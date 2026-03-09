use std::time::Instant;

use super::{SvgTraceResult, SvgTraceTimingsMs};

pub(super) fn finish_trace(svg: String, start: Instant) -> SvgTraceResult {
    let timings = SvgTraceTimingsMs {
        trace_total_ms: start.elapsed().as_millis() as u64,
        ..SvgTraceTimingsMs::default()
    };
    SvgTraceResult { svg, timings }
}
