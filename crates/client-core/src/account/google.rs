//! Desktop Google sign-in through the system browser and a loopback redirect (RFC 8252).
//!
//! The backend owns the PKCE verifier and the client secret; this side only binds the loopback listener, checks the authorization URL the backend built for it, and waits for the browser to deliver the authorization code with the matching `state`.

use super::validate::validate_google_login;
use super::*;
use std::net::{TcpListener, TcpStream};
use std::time::Instant;

/// How long the loopback listener waits for the browser to come back before the sign-in is abandoned.
pub const GOOGLE_SIGN_IN_TIMEOUT: Duration = Duration::from_secs(300);

const GOOGLE_AUTHORIZATION_PREFIX: &str = "https://accounts.google.com/";
const GOOGLE_CALLBACK_PATH: &str = "/callback";
const MAX_CALLBACK_REQUEST_BYTES: usize = 8 * 1024;
const MAX_STATE_BYTES: usize = 512;
const CALLBACK_IO_TIMEOUT: Duration = Duration::from_secs(5);
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub(super) fn google_loopback_target(port: u16) -> String {
    format!("http://127.0.0.1:{port}{GOOGLE_CALLBACK_PATH}")
}

/// Accepts exactly `http://127.0.0.1:<port>/callback` or `http://[::1]:<port>/callback` with a non-privileged port, the shape the backend accepts for the server-exchange flow.
pub(super) fn valid_google_loopback_target(target: &str) -> bool {
    let Some(rest) = target
        .strip_prefix("http://127.0.0.1:")
        .or_else(|| target.strip_prefix("http://[::1]:"))
    else {
        return false;
    };
    let Some(port) = rest.strip_suffix(GOOGLE_CALLBACK_PATH) else {
        return false;
    };
    !port.is_empty()
        && port.len() <= 5
        && !port.starts_with('0')
        && port.bytes().all(|byte| byte.is_ascii_digit())
        && port
            .parse::<u16>()
            .is_ok_and(|port| (1024..=u16::MAX).contains(&port))
}

/// Checks that the backend's authorization URL is a plain Google authorization URL redirecting to this listener, and returns its `state`. The URL is opened in the user's browser, so a malformed or foreign one is refused rather than opened.
pub(super) fn google_authorization_state(url: &str, target: &str) -> Result<String, AccountError> {
    if !crate::text::is_bounded_text(url, 4096)
        || !url.starts_with(GOOGLE_AUTHORIZATION_PREFIX)
        || url.bytes().any(|byte| {
            byte <= b' '
                || byte >= 0x7f
                || matches!(byte, b'"' | b'\'' | b'`' | b'|' | b'<' | b'>' | b'\\')
        })
    {
        return Err(AccountError::Unavailable);
    }
    let parsed = Url::parse(url).map_err(|_| AccountError::Unavailable)?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("accounts.google.com")
        || parsed.port().is_some()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
    {
        return Err(AccountError::Unavailable);
    }
    let redirect = single_query_value(&parsed, "redirect_uri").ok_or(AccountError::Unavailable)?;
    let state = single_query_value(&parsed, "state").ok_or(AccountError::Unavailable)?;
    if redirect != target || !valid_state(&state) {
        return Err(AccountError::Unavailable);
    }
    Ok(state)
}

fn single_query_value(url: &Url, name: &str) -> Option<String> {
    let mut values = url
        .query_pairs()
        .filter(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned());
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

fn valid_state(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_STATE_BYTES
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

/// What one loopback request means for the sign-in.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum GoogleCallback {
    /// Not the redirect, or not ours (another path such as `/favicon.ico`, or a `state` that does not match). The listener answers it and keeps waiting.
    Ignored,
    /// The redirect carried an authorization code for this sign-in.
    Code(String),
    /// The redirect ended the sign-in without a code.
    Failed(AccountError),
}

/// Interprets the request head of one loopback connection against the expected `state`.
pub(super) fn parse_google_callback(head: &str, expected_state: &str) -> GoogleCallback {
    let mut parts = head.lines().next().unwrap_or_default().split(' ');
    let (Some("GET"), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return GoogleCallback::Ignored;
    };
    if !target.starts_with('/') || !version.starts_with("HTTP/") {
        return GoogleCallback::Ignored;
    }
    let Ok(url) = Url::parse(&format!("http://127.0.0.1{target}")) else {
        return GoogleCallback::Ignored;
    };
    if url.path() != GOOGLE_CALLBACK_PATH {
        return GoogleCallback::Ignored;
    }
    let Some(state) = single_query_value(&url, "state") else {
        return GoogleCallback::Ignored;
    };
    if !constant_time_eq(state.as_bytes(), expected_state.as_bytes()) {
        return GoogleCallback::Ignored;
    }
    if url.query_pairs().any(|(key, _)| key == "error") {
        return GoogleCallback::Failed(AccountError::Cancelled);
    }
    match single_query_value(&url, "code") {
        Some(code) if validate_google_login("challenge", &code).is_ok() => {
            GoogleCallback::Code(code)
        }
        _ => GoogleCallback::Failed(AccountError::Unavailable),
    }
}

/// Waits on `listener` for the browser redirect carrying `state`, answering every request with a small page, and returns the authorization code. The wait ends with [`AccountError::Cancelled`] when the user denies access or `timeout` passes.
pub(super) fn receive_google_callback(
    listener: &TcpListener,
    state: &str,
    timeout: Duration,
) -> Result<String, AccountError> {
    listener
        .set_nonblocking(true)
        .map_err(|_| AccountError::Unavailable)?;
    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() >= deadline {
            return Err(AccountError::Cancelled);
        }
        let stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(ACCEPT_POLL_INTERVAL);
                continue;
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(AccountError::Unavailable),
        };
        match answer_callback(stream, state) {
            GoogleCallback::Ignored => continue,
            GoogleCallback::Code(code) => return Ok(code),
            GoogleCallback::Failed(error) => return Err(error),
        }
    }
}

fn answer_callback(mut stream: TcpStream, state: &str) -> GoogleCallback {
    let head = read_request_head(&mut stream);
    let outcome = head.as_deref().map_or(GoogleCallback::Ignored, |head| {
        parse_google_callback(head, state)
    });
    let (status, message) = match &outcome {
        GoogleCallback::Code(_) => ("200 OK", "Google 登录已完成，请返回水杉输入法。"),
        GoogleCallback::Failed(AccountError::Cancelled) => {
            ("200 OK", "已取消 Google 登录，请返回水杉输入法。")
        }
        GoogleCallback::Failed(_) => ("200 OK", "Google 登录未完成，请返回水杉输入法重试。"),
        GoogleCallback::Ignored => ("404 Not Found", "页面不存在。"),
    };
    let body = format!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>水杉输入法</title></head><body style=\"font-family:system-ui,sans-serif;margin:4rem auto;max-width:32rem;text-align:center\"><p>{message}</p></body></html>"
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    // The browser page is a courtesy; the outcome stands even if the browser already went away.
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
    outcome
}

fn read_request_head(stream: &mut TcpStream) -> Option<String> {
    stream.set_nonblocking(false).ok()?;
    stream.set_read_timeout(Some(CALLBACK_IO_TIMEOUT)).ok()?;
    stream.set_write_timeout(Some(CALLBACK_IO_TIMEOUT)).ok()?;
    let mut head = Vec::with_capacity(1024);
    let mut buffer = [0u8; 1024];
    while !head.windows(4).any(|window| window == b"\r\n\r\n") {
        if head.len() >= MAX_CALLBACK_REQUEST_BYTES {
            return None;
        }
        let read = stream.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        head.extend_from_slice(&buffer[..read]);
    }
    String::from_utf8(head).ok()
}
