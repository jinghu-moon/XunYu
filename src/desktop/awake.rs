use chrono::{DateTime, Duration, Local, NaiveTime, TimeZone};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use windows_sys::Win32::System::Power::{
    ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, EXECUTION_STATE,
    SetThreadExecutionState,
};

use crate::output::CliError;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AwakeMode {
    Off,
    Indefinite {
        display_on: bool,
    },
    Timed {
        display_on: bool,
        until: DateTime<Local>,
    },
}

impl AwakeMode {
    pub(crate) fn display_str(&self) -> String {
        match self {
            Self::Off => "未激活".into(),
            Self::Indefinite { display_on } => {
                if *display_on {
                    "持续唤醒（保持屏幕亮）".into()
                } else {
                    "持续唤醒".into()
                }
            }
            Self::Timed { display_on, until } => {
                let now = Local::now();
                let remaining = *until - now;
                let h = remaining.num_hours();
                let m = remaining.num_minutes() % 60;
                let suffix = if *display_on {
                    "，保持屏幕亮"
                } else {
                    ""
                };
                format!(
                    "定时唤醒，剩余 {}h {}m（到 {}{}）",
                    h,
                    m,
                    until.format("%H:%M:%S"),
                    suffix
                )
            }
        }
    }
}

pub(crate) struct AwakeState {
    pub(crate) mode: AwakeMode,
    cancel_tx: Option<Sender<()>>,
}

impl AwakeState {
    pub(crate) fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            mode: AwakeMode::Off,
            cancel_tx: None,
        }))
    }
}

pub(crate) fn awake_indefinite(
    display_on: bool,
    state: &Arc<Mutex<AwakeState>>,
) -> Result<(), CliError> {
    cancel_awake(state);
    let flags = build_flags(display_on);
    unsafe {
        SetThreadExecutionState(flags);
    }

    let (tx, rx) = channel::<()>();
    let flags_cancel = ES_CONTINUOUS;

    std::thread::spawn(move || {
        unsafe {
            SetThreadExecutionState(flags);
        }
        let _ = rx.recv();
        unsafe {
            SetThreadExecutionState(flags_cancel);
        }
    });

    let mut s = state.lock().unwrap();
    s.mode = AwakeMode::Indefinite { display_on };
    s.cancel_tx = Some(tx);
    Ok(())
}

pub(crate) fn awake_timed(
    display_on: bool,
    duration: std::time::Duration,
    state: &Arc<Mutex<AwakeState>>,
    expired_callback: impl Fn() + Send + 'static,
) -> Result<(), CliError> {
    cancel_awake(state);
    let until = Local::now() + Duration::from_std(duration).unwrap();
    let flags = build_flags(display_on);
    let (tx, rx) = channel::<()>();

    std::thread::spawn(move || {
        unsafe {
            SetThreadExecutionState(flags);
        }
        let result = rx.recv_timeout(duration);
        unsafe {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
        if result.is_err() {
            expired_callback();
        }
    });

    let mut s = state.lock().unwrap();
    s.mode = AwakeMode::Timed { display_on, until };
    s.cancel_tx = Some(tx);
    Ok(())
}

pub(crate) fn cancel_awake(state: &Arc<Mutex<AwakeState>>) {
    let mut s = state.lock().unwrap();
    if let Some(tx) = s.cancel_tx.take() {
        let _ = tx.send(());
    }
    s.mode = AwakeMode::Off;
}

pub(crate) fn parse_expire_at(time_str: &str) -> Result<std::time::Duration, CliError> {
    let t = NaiveTime::parse_from_str(time_str, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(time_str, "%H:%M:%S"))
        .map_err(|_| CliError::new(2, format!("时间格式无效：{time_str}（应为 HH:MM）")))?;

    let now = Local::now();
    let today = now.date_naive();
    let mut target = Local
        .from_local_datetime(&today.and_time(t))
        .single()
        .ok_or_else(|| CliError::new(2, "时区转换失败"))?;

    if target <= now {
        target = target + Duration::days(1);
    }

    let diff = (target - now)
        .to_std()
        .map_err(|_| CliError::new(2, "时间计算错误"))?;
    Ok(diff)
}

pub(crate) fn parse_duration(s: &str) -> Result<std::time::Duration, CliError> {
    let s = s.trim().to_lowercase();
    let mut total_secs: u64 = 0;
    let mut num_buf = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() {
            num_buf.push(c);
        } else {
            let n: u64 = num_buf
                .parse()
                .map_err(|_| CliError::new(2, format!("时长格式无效：{s}")))?;
            num_buf.clear();
            match c {
                'h' => total_secs += n * 3600,
                'm' => total_secs += n * 60,
                's' => total_secs += n,
                _ => return Err(CliError::new(2, format!("时长单位无效：{c}"))),
            }
        }
    }
    if !num_buf.is_empty() {
        let n: u64 = num_buf
            .parse()
            .map_err(|_| CliError::new(2, format!("时长格式无效：{s}")))?;
        total_secs += n * 60;
    }

    if total_secs == 0 {
        return Err(CliError::new(2, format!("时长为零：{s}")));
    }
    Ok(std::time::Duration::from_secs(total_secs))
}

fn build_flags(display_on: bool) -> EXECUTION_STATE {
    if display_on {
        ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED
    } else {
        ES_CONTINUOUS | ES_SYSTEM_REQUIRED
    }
}
