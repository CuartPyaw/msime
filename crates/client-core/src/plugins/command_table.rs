//! `kind = "command_table"`: `/` commands, each a trigger, a title and a template.
//!
//! ```toml
//! [[commands]]
//! trigger = "sig"
//! title = "签名"
//! template = "{date:%Y-%m-%d} 张三"
//! ```
//!
//! A template is literal text and three placeholders: `{date}` or `{date:FORMAT}`, `{time}` or `{time:FORMAT}`, and `{weekday}`, with FORMAT a strftime description. Nothing else - no clipboard, no environment, no nesting - so a table can only ever produce text. The Engine expands templates (`crates/engine/src/local/command.rs`) and silently drops a row it cannot use; the rules are repeated here, where client-core cannot reach the Engine, so a pack is refused with the reason instead of losing rows nobody is told about.

use serde::Serialize;
use time::format_description::parse_strftime_borrowed;
use time::{Date, Month, PrimitiveDateTime, Time};
use toml::Value;

use super::{only_keys, PluginKind};

pub(crate) const MANIFEST_KEYS: [&str; 1] = ["commands"];

/// Commands in one pack, and in every enabled pack together: the Engine keeps no more.
pub const MAX_COMMANDS: usize = 256;
/// Trigger letters.
pub const MAX_TRIGGER_BYTES: usize = 32;
/// Title shown beside a row, in bytes.
pub const MAX_TITLE_BYTES: usize = 48;
/// A template, and the text it expands to, in UTF-16 units: the Windows candidate pipe's text field, the Engine's `TEXT_UTF16_LIMIT`.
pub const MAX_TEXT_UTF16: usize = 199;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandRow {
    /// Lowercase ASCII letters typed after `/`.
    pub trigger: String,
    pub title: String,
    pub template: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandTable {
    pub commands: Vec<CommandRow>,
}

pub(crate) fn parse(table: &toml::map::Map<String, Value>) -> Result<CommandTable, String> {
    let items = table
        .get("commands")
        .and_then(Value::as_array)
        .ok_or("command table needs commands")?;
    if items.is_empty() || items.len() > MAX_COMMANDS {
        return Err("command table has too few or too many commands".into());
    }
    let mut commands: Vec<CommandRow> = Vec::with_capacity(items.len());
    for item in items {
        let row = item.as_table().ok_or("each command must be a table")?;
        only_keys(row, &["trigger", "title", "template"], "a command")?;
        let field = |key: &str| {
            row.get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| format!("each command needs a {key}"))
        };
        let command = CommandRow {
            trigger: field("trigger")?,
            title: field("title")?,
            template: field("template")?,
        };
        validate(&command)?;
        if commands.iter().any(|kept| kept.trigger == command.trigger) {
            return Err(format!("trigger {} is listed twice", command.trigger));
        }
        commands.push(command);
    }
    Ok(CommandTable { commands })
}

/// Why the Engine could not use `command`, if it could not.
pub fn validate(command: &CommandRow) -> Result<(), String> {
    let trigger = &command.trigger;
    if trigger.is_empty()
        || trigger.len() > MAX_TRIGGER_BYTES
        || !trigger.bytes().all(|byte| byte.is_ascii_lowercase())
    {
        return Err(format!(
            "trigger {trigger} must be 1 to 32 lowercase letters"
        ));
    }
    if command.title.trim().is_empty()
        || !crate::text::is_bounded_text(&command.title, MAX_TITLE_BYTES)
    {
        return Err(format!("the title of {trigger} is empty or too long"));
    }
    let template = &command.template;
    if template.trim().is_empty()
        || crate::text::has_disallowed_control(template)
        || !crate::text::is_bounded_utf16(template, MAX_TEXT_UTF16)
    {
        return Err(format!("the template of {trigger} is empty or too long"));
    }
    let expanded = expand_longest(template)
        .ok_or_else(|| format!("the template of {trigger} has an unknown placeholder"))?;
    if !crate::text::is_bounded_utf16(&expanded, MAX_TEXT_UTF16) {
        return Err(format!("the template of {trigger} expands past the limit"));
    }
    Ok(())
}

/// The template expanded at an instant whose fields are all at their widest - a two-digit day and hour, September and Wednesday for the longest English names - so a template that fits here fits on every day. `None` when a placeholder is not one of the three or its format does not parse.
fn expand_longest(template: &str) -> Option<String> {
    let instant = PrimitiveDateTime::new(
        Date::from_calendar_date(2026, Month::September, 30).ok()?,
        Time::from_hms(23, 59, 59).ok()?,
    );
    let mut output = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find(['{', '}']) {
        if rest.as_bytes()[open] == b'}' {
            return None;
        }
        output.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let close = after.find(['{', '}'])?;
        if after.as_bytes()[close] == b'{' {
            return None;
        }
        let placeholder = &after[..close];
        rest = &after[close + 1..];
        if placeholder == "weekday" {
            output.push_str("星期三");
            continue;
        }
        let format = match placeholder.split_once(':').unwrap_or((placeholder, "")) {
            ("date", "") => "%Y-%m-%d",
            ("time", "") => "%H:%M",
            ("date" | "time", format) => format,
            _ => return None,
        };
        let items = parse_strftime_borrowed(format).ok()?;
        output.push_str(&instant.format(&items).ok()?);
    }
    output.push_str(rest);
    Some(output)
}

/// The rows of the enabled command-table packs under `root`, in the order the ids are listed, keeping the first command of each trigger and at most `MAX_COMMANDS`: the table a host hands to the Engine. A pack that is missing or does not load contributes nothing; the settings page reports it.
pub fn enabled_commands(root: &std::path::Path, enabled: &[String]) -> Vec<CommandRow> {
    let mut rows: Vec<CommandRow> = Vec::new();
    for id in enabled {
        let Ok(package) = super::load_package(root, None, PluginKind::CommandTable, id) else {
            continue;
        };
        let super::PluginContent::CommandTable(table) = package.content else {
            continue;
        };
        for row in table.commands {
            if rows.len() == MAX_COMMANDS {
                return rows;
            }
            if !rows.iter().any(|kept| kept.trigger == row.trigger) {
                rows.push(row);
            }
        }
    }
    rows
}
