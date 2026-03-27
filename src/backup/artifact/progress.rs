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

pub(crate) fn emit_progress_snapshot(
    enabled: bool,
    phase: ExportProgressPhase,
    selected_files: usize,
    processed_files: usize,
    bytes_in: u64,
    bytes_out: u64,
    throughput: u64,
    elapsed_ms: u128,
) {
    if !enabled {
        return;
    }
    emit_progress_event(&ExportProgressEvent {
        phase,
        selected_files,
        processed_files,
        bytes_in,
        bytes_out,
        throughput,
        elapsed_ms,
    });
}

pub(crate) fn emit_read_progress(
    enabled: bool,
    selected_files: usize,
    bytes_in: u64,
    elapsed_ms: u128,
) {
    emit_progress_snapshot(
        enabled,
        ExportProgressPhase::Read,
        selected_files,
        selected_files,
        bytes_in,
        0,
        0,
        elapsed_ms,
    );
}

pub(crate) fn emit_compress_progress(
    enabled: bool,
    selected_files: usize,
    bytes_in: u64,
    elapsed_ms: u128,
) {
    emit_progress_snapshot(
        enabled,
        ExportProgressPhase::Compress,
        selected_files,
        0,
        bytes_in,
        0,
        0,
        elapsed_ms,
    );
}

pub(crate) fn emit_write_progress(
    enabled: bool,
    selected_files: usize,
    processed_files: usize,
    bytes_in: u64,
    bytes_out: u64,
    throughput: u64,
    elapsed_ms: u128,
) {
    emit_progress_snapshot(
        enabled,
        ExportProgressPhase::Write,
        selected_files,
        processed_files,
        bytes_in,
        bytes_out,
        throughput,
        elapsed_ms,
    );
}

pub(crate) fn emit_verify_source_progress(enabled: bool, elapsed_ms: u128) {
    emit_progress_snapshot(
        enabled,
        ExportProgressPhase::VerifySource,
        0,
        0,
        0,
        0,
        0,
        elapsed_ms,
    );
}

pub(crate) fn emit_verify_output_progress(
    enabled: bool,
    selected_files: usize,
    processed_files: usize,
    bytes_in: u64,
    bytes_out: u64,
    elapsed_ms: u128,
) {
    emit_progress_snapshot(
        enabled,
        ExportProgressPhase::VerifyOutput,
        selected_files,
        processed_files,
        bytes_in,
        bytes_out,
        0,
        elapsed_ms,
    );
}

#[cfg(test)]
mod tests {
    use super::{
        ExportProgressEvent, ExportProgressPhase, emit_compress_progress, emit_progress_snapshot,
        emit_read_progress, emit_verify_output_progress, emit_verify_source_progress,
        emit_write_progress,
    };

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

    #[test]
    fn helper_emitters_are_noop_when_disabled() {
        emit_progress_snapshot(false, ExportProgressPhase::Read, 1, 1, 2, 3, 4, 5);
        emit_verify_source_progress(false, 1);
        emit_read_progress(false, 1, 2, 3);
        emit_compress_progress(false, 1, 2, 3);
        emit_write_progress(false, 1, 1, 2, 3, 4, 5);
        emit_verify_output_progress(false, 1, 1, 2, 3, 4);
    }
}
