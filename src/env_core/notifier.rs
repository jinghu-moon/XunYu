use std::process::Command;

use super::config::EnvCoreConfig;
use super::types::{EnvError, EnvResult};

pub fn notify_run_result(
    cfg: &EnvCoreConfig,
    command_line: &str,
    exit_code: Option<i32>,
    success: bool,
) -> EnvResult<bool> {
    if !cfg.notify_enabled || is_notify_suppressed() {
        return Ok(false);
    }

    let title = if success {
        "xun env run succeeded"
    } else {
        "xun env run failed"
    };
    let body = format!(
        "{} (exit={})",
        command_line,
        exit_code
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    send_notification(title, &body)
}

fn is_notify_suppressed() -> bool {
    std::env::var("ENVMGR_NO_NOTIFY")
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn send_notification(title: &str, body: &str) -> EnvResult<bool> {
    #[cfg(target_os = "windows")]
    {
        if try_windows_toast(title, body) {
            return Ok(true);
        }
        if try_run("msg", &["*", title, body]) {
            return Ok(true);
        }
        return Err(EnvError::Other(
            "failed to send windows notification".to_string(),
        ));
    }

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            escape_applescript(body),
            escape_applescript(title)
        );
        if try_run("osascript", &["-e", &script]) {
            return Ok(true);
        }
        return Err(EnvError::Other(
            "failed to send macOS notification".to_string(),
        ));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if try_run("notify-send", &[title, body]) {
            return Ok(true);
        }
        return Err(EnvError::Other(
            "failed to send linux notification".to_string(),
        ));
    }

    #[allow(unreachable_code)]
    Err(EnvError::UnsupportedPlatform)
}

#[cfg(target_os = "windows")]
fn try_windows_toast(title: &str, body: &str) -> bool {
    let script = format!(
        "$ErrorActionPreference='Stop';\
        [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null;\
        [Windows.UI.Notifications.ToastNotification, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null;\
        [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] > $null;\
        $xml = New-Object Windows.Data.Xml.Dom.XmlDocument;\
        $xml.LoadXml('<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>');\
        $toast = [Windows.UI.Notifications.ToastNotification]::new($xml);\
        $app = '{{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}}\\WindowsPowerShell\\v1.0\\powershell.exe';\
        [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier($app).Show($toast);",
        escape_xml(title),
        escape_xml(body)
    );
    try_run("powershell", &["-NoProfile", "-Command", &script])
}

fn try_run(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn escape_xml(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(target_os = "macos")]
fn escape_applescript(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}
