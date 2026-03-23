use serde::{Deserialize, Serialize};

use crate::backup_formats::ProgressMode;
use crate::output::can_interact;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportProgressPhase {
    VerifySource,
    Read,
    Compress,
    Write,
    VerifyOutput,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportProgressEvent {
    pub phase: ExportProgressPhase,
    pub selected_files: usize,
    pub processed_files: usize,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub throughput: u64,
    pub elapsed_ms: u128,
}

pub(crate) fn should_emit_progress(mode: ProgressMode, json: bool) -> bool {
    if json {
        return false;
    }
    match mode {
        ProgressMode::Always => true,
        ProgressMode::Off => false,
        ProgressMode::Auto => can_interact(),
    }
}

pub(crate) fn emit_progress_event(event: &ExportProgressEvent) {
    eprintln!(
        "progress: phase={} files={}/{} bytes_in={} bytes_out={} throughput={} elapsed={}ms",
        serde_json::to_string(&event.phase)
            .unwrap_or_else(|_| "\"unknown\"".to_string())
            .trim_matches('"'),
        event.processed_files,
        event.selected_files,
        event.bytes_in,
        event.bytes_out,
        event.throughput,
        event.elapsed_ms
    );
}

#[cfg(test)]
mod tests {
    use super::{ExportProgressEvent, ExportProgressPhase};

    #[test]
    fn export_progress_event_contains_contract_fields() {
        let event = ExportProgressEvent {
            phase: ExportProgressPhase::Write,
            selected_files: 10,
            processed_files: 4,
            bytes_in: 1024,
            bytes_out: 512,
            throughput: 256,
            elapsed_ms: 42,
        };

        assert_eq!(event.phase, ExportProgressPhase::Write);
        assert_eq!(event.selected_files, 10);
        assert_eq!(event.processed_files, 4);
        assert_eq!(event.bytes_in, 1024);
        assert_eq!(event.bytes_out, 512);
        assert_eq!(event.throughput, 256);
        assert_eq!(event.elapsed_ms, 42);
    }

    #[test]
    fn export_progress_phase_serialization_is_stable() {
        let phases = [
            ExportProgressPhase::VerifySource,
            ExportProgressPhase::Read,
            ExportProgressPhase::Compress,
            ExportProgressPhase::Write,
            ExportProgressPhase::VerifyOutput,
        ];
        let encoded = serde_json::to_string(&phases).unwrap();
        assert_eq!(
            encoded,
            r#"["verify_source","read","compress","write","verify_output"]"#
        );
    }
}
