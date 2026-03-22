use serde::{Deserialize, Serialize};
use windows_sys::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum LayoutTemplate {
    Grid(GridLayout),
}

impl Default for LayoutTemplate {
    fn default() -> Self {
        Self::Grid(GridLayout::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct GridLayout {
    #[serde(default = "default_2")]
    pub(crate) rows: usize,
    #[serde(default = "default_2")]
    pub(crate) cols: usize,
    #[serde(default = "default_gap")]
    pub(crate) gap: i32,
    #[serde(default)]
    pub(crate) padding: i32,
    #[serde(default)]
    pub(crate) monitor: usize,
}

impl Default for GridLayout {
    fn default() -> Self {
        Self {
            rows: 2,
            cols: 2,
            gap: 8,
            padding: 0,
            monitor: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ZoneRect {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) w: i32,
    pub(crate) h: i32,
}

impl ZoneRect {
    pub(crate) fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MonitorArea {
    pub(crate) index: usize,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) w: i32,
    pub(crate) h: i32,
}

unsafe extern "system" fn monitor_callback(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let monitors = unsafe { &mut *(lparam as *mut Vec<MonitorArea>) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        rcMonitor: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        rcWork: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        dwFlags: 0,
    };
    let ok = unsafe { GetMonitorInfoW(hmonitor, &mut info) };
    if ok != 0 {
        let index = monitors.len();
        monitors.push(MonitorArea {
            index,
            x: info.rcWork.left,
            y: info.rcWork.top,
            w: info.rcWork.right - info.rcWork.left,
            h: info.rcWork.bottom - info.rcWork.top,
        });
    }
    1
}

pub(crate) fn enumerate_monitors() -> Vec<MonitorArea> {
    let mut monitors: Vec<MonitorArea> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            HDC::default(),
            std::ptr::null_mut(),
            Some(monitor_callback),
            &mut monitors as *mut _ as LPARAM,
        );
    }
    monitors.sort_by_key(|m| m.x);
    for (i, monitor) in monitors.iter_mut().enumerate() {
        monitor.index = i;
    }
    monitors
}

pub(crate) fn get_monitor_area(index: usize) -> Option<MonitorArea> {
    enumerate_monitors().into_iter().find(|m| m.index == index)
}

pub(crate) fn compute_zones(template: &LayoutTemplate, monitor: &MonitorArea) -> Vec<ZoneRect> {
    match template {
        LayoutTemplate::Grid(layout) => compute_grid_zones(layout, monitor),
    }
}

fn compute_grid_zones(layout: &GridLayout, monitor: &MonitorArea) -> Vec<ZoneRect> {
    let rows = layout.rows.max(1);
    let cols = layout.cols.max(1);
    let gap = layout.gap.max(0);
    let padding = layout.padding.max(0);

    let total_gap_x = gap * (cols as i32 - 1).max(0);
    let total_gap_y = gap * (rows as i32 - 1).max(0);

    let usable_w = (monitor.w - padding * 2 - total_gap_x).max(1);
    let usable_h = (monitor.h - padding * 2 - total_gap_y).max(1);

    let cell_w = usable_w / cols as i32;
    let cell_h = usable_h / rows as i32;

    let mut zones = Vec::with_capacity(rows * cols);
    for r in 0..rows {
        for c in 0..cols {
            let x = monitor.x + padding + c as i32 * (cell_w + gap);
            let y = monitor.y + padding + r as i32 * (cell_h + gap);
            zones.push(ZoneRect::new(x, y, cell_w, cell_h));
        }
    }
    zones
}

pub(crate) fn preview_ascii(zones: &[ZoneRect], monitor: &MonitorArea) -> String {
    if zones.is_empty() {
        return "[empty layout]".to_string();
    }
    let cols = estimate_columns(zones);
    let mut out = String::new();
    out.push_str(&format!(
        "Monitor {} ({}x{})\n",
        monitor.index, monitor.w, monitor.h
    ));
    for (idx, zone) in zones.iter().enumerate() {
        out.push_str(&format!(
            "  #{:<2} x={:<4} y={:<4} w={:<4} h={:<4}\n",
            idx + 1,
            zone.x,
            zone.y,
            zone.w,
            zone.h
        ));
        if cols > 0 && (idx + 1) % cols == 0 {
            out.push('\n');
        }
    }
    out
}

fn estimate_columns(zones: &[ZoneRect]) -> usize {
    if zones.len() <= 1 {
        return zones.len();
    }
    let first_y = zones[0].y;
    zones.iter().take_while(|z| z.y == first_y).count().max(1)
}

fn default_2() -> usize {
    2
}

fn default_gap() -> i32 {
    8
}
