// 不碰剪贴板，在任何主机上检验设置应用侧的隐私判定，并对照 Server 侧的同一份格式名单。
#[path = "../src/clipboard_privacy.rs"]
mod clipboard_privacy;

use clipboard_privacy::{
    ClipboardPermission, ClipboardPrivacyMarkers, CLOUD_PERMISSION_FORMAT, EXCLUDE_MONITOR_FORMAT,
    HISTORY_PERMISSION_FORMAT, VIEWER_IGNORE_FORMAT,
};

const SERVER_POLICY: &str =
    include_str!("../../../platforms/windows/src/clipboard/ClipboardPrivacyPolicy.h");

#[test]
fn format_names_match_the_server_monitor() {
    for name in [
        EXCLUDE_MONITOR_FORMAT,
        VIEWER_IGNORE_FORMAT,
        HISTORY_PERMISSION_FORMAT,
        CLOUD_PERMISSION_FORMAT,
    ] {
        assert!(
            SERVER_POLICY.contains(&format!("L\"{name}\"")),
            "{name} is missing from ClipboardPrivacyPolicy.h"
        );
    }
}

#[test]
fn permission_dword_zero_denies_and_unreadable_denies() {
    assert_eq!(
        ClipboardPermission::from_data(Some(&0u32.to_le_bytes())),
        ClipboardPermission::Denied
    );
    assert_eq!(
        ClipboardPermission::from_data(Some(&1u32.to_le_bytes())),
        ClipboardPermission::Allowed
    );
    assert_eq!(
        ClipboardPermission::from_data(None),
        ClipboardPermission::Denied
    );
    assert_eq!(
        ClipboardPermission::from_data(Some(&[1, 0])),
        ClipboardPermission::Denied
    );
}

#[test]
fn any_privacy_marker_excludes_the_sample() {
    assert!(!ClipboardPrivacyMarkers::default().excluded());
    for markers in [
        ClipboardPrivacyMarkers {
            exclude_from_monitor: true,
            ..Default::default()
        },
        ClipboardPrivacyMarkers {
            viewer_ignore: true,
            ..Default::default()
        },
        ClipboardPrivacyMarkers {
            history: ClipboardPermission::Denied,
            ..Default::default()
        },
        ClipboardPrivacyMarkers {
            cloud: ClipboardPermission::Denied,
            ..Default::default()
        },
    ] {
        assert!(markers.excluded(), "{markers:?}");
    }
    assert!(!ClipboardPrivacyMarkers {
        history: ClipboardPermission::Allowed,
        cloud: ClipboardPermission::Allowed,
        ..Default::default()
    }
    .excluded());
}
