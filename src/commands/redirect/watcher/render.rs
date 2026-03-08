use super::super::engine::{RedirectOptions, RedirectResult};

pub(super) fn render_batch(results: &[RedirectResult], opts: &RedirectOptions) {
    if results.is_empty() {
        return;
    }
    match opts.format {
        crate::model::ListFormat::Json => {
            let arr: Vec<serde_json::Value> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "action": r.action,
                        "src": r.src,
                        "dst": r.dst,
                        "rule": r.rule,
                        "result": r.result,
                        "reason": r.reason,
                    })
                })
                .collect();
            out_println!("{}", serde_json::Value::Array(arr));
        }
        _ => {
            for r in results {
                out_println!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    r.action,
                    r.src,
                    r.dst,
                    r.rule,
                    r.result,
                    r.reason
                );
            }
        }
    }
}
