// Desktop window control.

use crate::output::{CliError, CliResult};
use crate::proc;
use crate::windows::window_api;

#[derive(Debug, Clone, Copy)]
pub(crate) enum TopmostMode {
    Enable,
    Disable,
}

pub(crate) struct WindowTarget {
    pub(crate) pid: u32,
    pub(crate) hwnd: isize,
}

pub(crate) fn resolve_window_target(app: Option<&str>, title: Option<&str>) -> Result<WindowTarget, CliError> {
    let Some(target) = pick_window(app, title) else {
        return Err(CliError::new(2, "No matching window found."));
    };
    Ok(target)
}

fn pick_window(app: Option<&str>, title: Option<&str>) -> Option<WindowTarget> {
    let app = app.map(|value| value.trim()).filter(|value| !value.is_empty());
    let title = title.map(|value| value.trim()).filter(|value| !value.is_empty());

    if app.is_none() && title.is_none() {
        return proc::list_all(false)
            .into_iter()
            .find(|p| !p.window_title.trim().is_empty())
            .map(|p| WindowTarget {
                pid: p.pid,
                hwnd: 0,
            });
    }

    if let Some(needle) = title {
        return proc::find_by_window_title(needle)
            .into_iter()
            .next()
            .map(|p| WindowTarget {
                pid: p.pid,
                hwnd: 0,
            });
    }

    let needle = app.unwrap_or_default();
    proc::find_by_name(needle)
        .into_iter()
        .next()
        .map(|p| WindowTarget {
            pid: p.pid,
            hwnd: 0,
        })
}

pub(crate) fn focus_window(target: &WindowTarget) -> CliResult {
    let hwnd = ensure_hwnd(target)?;
    window_api::focus_window(hwnd).map_err(|e| map_window_error("focus", e))
}

pub(crate) fn move_window(target: &WindowTarget, x: i32, y: i32) -> CliResult {
    let hwnd = ensure_hwnd(target)?;
    window_api::move_window(hwnd, x, y).map_err(|e| map_window_error("move", e))
}

pub(crate) fn resize_window(target: &WindowTarget, width: i32, height: i32) -> CliResult {
    let hwnd = ensure_hwnd(target)?;
    window_api::resize_window(hwnd, width, height).map_err(|e| map_window_error("resize", e))
}

pub(crate) fn set_transparency(target: &WindowTarget, alpha: u8) -> CliResult {
    let hwnd = ensure_hwnd(target)?;
    window_api::set_transparency(hwnd, alpha).map_err(|e| map_window_error("transparent", e))
}

pub(crate) fn set_topmost(target: &WindowTarget, mode: TopmostMode) -> CliResult {
    let hwnd = ensure_hwnd(target)?;
    let enable = matches!(mode, TopmostMode::Enable);
    window_api::set_topmost(hwnd, enable).map_err(|e| map_window_error("top", e))
}

fn ensure_hwnd(target: &WindowTarget) -> Result<isize, CliError> {
    if target.hwnd != 0 {
        return Ok(target.hwnd);
    }
    window_api::find_hwnd_by_pid(target.pid).map_err(|e| map_window_error("find", e))
}

fn map_window_error(action: &str, err: window_api::WindowApiError) -> CliError {
    match err {
        window_api::WindowApiError::NotFound => CliError::new(2, "No matching window found."),
        window_api::WindowApiError::OsError { action: op, code } => CliError::with_details(
            2,
            format!("Failed to {action} window."),
            &[format!("Win32: {op} (code={code})"), "Fix: retry with admin rights.".to_string()],
        ),
    }
}
