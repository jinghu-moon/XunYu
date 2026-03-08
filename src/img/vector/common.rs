use std::time::Instant;

use super::{SvgTraceResult, SvgTraceTimingsMs};

pub(super) fn finish_trace(svg: String, start: Instant) -> SvgTraceResult {
    let mut timings = SvgTraceTimingsMs::default();
    timings.trace_total_ms = start.elapsed().as_millis() as u64;
    SvgTraceResult { svg, timings }
}
