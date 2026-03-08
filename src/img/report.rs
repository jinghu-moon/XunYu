use comfy_table::{Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL};

use super::types::{ProcessResult, format_bytes};

pub fn print_summary(results: &[ProcessResult], wall_ms: u64) {
    let total = results.len();
    let success: Vec<_> = results
        .iter()
        .filter(|r| r.error.is_none() && !r.skipped)
        .collect();
    let skipped: Vec<_> = results.iter().filter(|r| r.skipped).collect();
    let failed: Vec<_> = results.iter().filter(|r| r.error.is_some()).collect();

    let total_in: u64 = success.iter().map(|r| r.input_bytes).sum();
    let total_out: u64 = success.iter().map(|r| r.output_bytes).sum();
    let total_elapsed_ms: u64 = success.iter().map(|r| r.elapsed_ms).sum();
    let stage_read_ms: u64 = success.iter().map(|r| r.stage_ms.read_ms).sum();
    let stage_decode_ms: u64 = success.iter().map(|r| r.stage_ms.decode_ms).sum();
    let stage_resize_ms: u64 = success.iter().map(|r| r.stage_ms.resize_ms).sum();
    let stage_pixel_convert_ms: u64 = success.iter().map(|r| r.stage_ms.pixel_convert_ms).sum();
    let stage_encode_pre_ms: u64 = success.iter().map(|r| r.stage_ms.encode_pre_ms).sum();
    let stage_png_optimize_ms: u64 = success.iter().map(|r| r.stage_ms.png_optimize_ms).sum();
    let stage_codec_ms: u64 = success.iter().map(|r| r.stage_ms.codec_ms).sum();
    let stage_write_ms: u64 = success.iter().map(|r| r.stage_ms.write_ms).sum();
    let stage_svg_trace_ms: u64 = success.iter().map(|r| r.stage_ms.svg_trace_ms).sum();
    let stage_svg_serialize_ms: u64 = success.iter().map(|r| r.stage_ms.svg_serialize_ms).sum();
    let stage_svg_trace_internal_ms: u64 = success
        .iter()
        .map(|r| r.stage_ms.svg_trace_internal_ms)
        .sum();
    let stage_svg_vc_to_color_ms: u64 = success.iter().map(|r| r.stage_ms.svg_vc_to_color_ms).sum();
    let stage_svg_vc_keying_ms: u64 = success.iter().map(|r| r.stage_ms.svg_vc_keying_ms).sum();
    let stage_svg_vc_cluster_ms: u64 = success.iter().map(|r| r.stage_ms.svg_vc_cluster_ms).sum();
    let stage_svg_vc_cluster_quantize_ms: u64 = success
        .iter()
        .map(|r| r.stage_ms.svg_vc_cluster_quantize_ms)
        .sum();
    let stage_svg_vc_cluster_label_ms: u64 = success
        .iter()
        .map(|r| r.stage_ms.svg_vc_cluster_label_ms)
        .sum();
    let stage_svg_vc_cluster_stats_ms: u64 = success
        .iter()
        .map(|r| r.stage_ms.svg_vc_cluster_stats_ms)
        .sum();
    let stage_svg_vc_cluster_merge_ms: u64 = success
        .iter()
        .map(|r| r.stage_ms.svg_vc_cluster_merge_ms)
        .sum();
    let stage_svg_vc_cluster_finalize_ms: u64 = success
        .iter()
        .map(|r| r.stage_ms.svg_vc_cluster_finalize_ms)
        .sum();
    let stage_svg_vc_path_build_ms: u64 = success
        .iter()
        .map(|r| r.stage_ms.svg_vc_path_build_ms)
        .sum();
    let stage_svg_vc_path_sort_ms: u64 =
        success.iter().map(|r| r.stage_ms.svg_vc_path_sort_ms).sum();
    let stage_svg_vc_path_trace_ms: u64 = success
        .iter()
        .map(|r| r.stage_ms.svg_vc_path_trace_ms)
        .sum();
    let stage_svg_vc_path_smooth_ms: u64 = success
        .iter()
        .map(|r| r.stage_ms.svg_vc_path_smooth_ms)
        .sum();
    let stage_svg_vc_path_svg_emit_ms: u64 = success
        .iter()
        .map(|r| r.stage_ms.svg_vc_path_svg_emit_ms)
        .sum();
    let stage_svg_vc_path_components_total: u64 = success
        .iter()
        .map(|r| r.stage_ms.svg_vc_path_components_total)
        .sum();
    let stage_svg_vc_path_components_simplified: u64 = success
        .iter()
        .map(|r| r.stage_ms.svg_vc_path_components_simplified)
        .sum();
    let stage_svg_vc_path_components_smoothed: u64 = success
        .iter()
        .map(|r| r.stage_ms.svg_vc_path_components_smoothed)
        .sum();
    let stage_svg_vc_wrap_ms: u64 = success.iter().map(|r| r.stage_ms.svg_vc_wrap_ms).sum();
    let stage_total_ms = stage_read_ms
        + stage_decode_ms
        + stage_resize_ms
        + stage_pixel_convert_ms
        + stage_encode_pre_ms
        + stage_png_optimize_ms
        + stage_codec_ms
        + stage_write_ms;
    let savings = if total_in > 0 {
        100.0 - (total_out as f64 / total_in as f64 * 100.0)
    } else {
        0.0
    };
    let throughput_mb = if wall_ms > 0 {
        total_in as f64 / 1024.0 / 1024.0 / (wall_ms as f64 / 1000.0)
    } else {
        0.0
    };

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS);
    table.set_header(vec!["指标", "值"]);
    table.add_row(vec!["总文件数", &total.to_string()]);
    table.add_row(vec!["成功", &success.len().to_string()]);
    table.add_row(vec!["跳过", &skipped.len().to_string()]);
    table.add_row(vec!["失败", &failed.len().to_string()]);
    table.add_row(vec!["输入总大小", &format_bytes(total_in)]);
    table.add_row(vec!["输出总大小", &format_bytes(total_out)]);
    table.add_row(vec!["节省空间", &format!("{savings:.1}%")]);
    table.add_row(vec!["吞吐量", &format!("{throughput_mb:.1} MB/s")]);
    let avg_file_ms = if !success.is_empty() {
        total_elapsed_ms as f64 / success.len() as f64
    } else {
        0.0
    };
    let stage_avg_ms = if !success.is_empty() {
        stage_total_ms as f64 / success.len() as f64
    } else {
        0.0
    };
    let stage_pct = |part: u64| {
        if stage_total_ms > 0 {
            part as f64 / stage_total_ms as f64 * 100.0
        } else {
            0.0
        }
    };

    table.add_row(vec!["平均单图耗时", &format!("{avg_file_ms:.1} ms")]);
    table.add_row(vec!["阶段累计耗时", &format!("{stage_total_ms} ms")]);
    table.add_row(vec![
        "读文件阶段",
        &format!("{stage_read_ms} ms ({:.1}%)", stage_pct(stage_read_ms)),
    ]);
    table.add_row(vec![
        "解码阶段",
        &format!("{stage_decode_ms} ms ({:.1}%)", stage_pct(stage_decode_ms)),
    ]);
    table.add_row(vec![
        "缩放阶段",
        &format!("{stage_resize_ms} ms ({:.1}%)", stage_pct(stage_resize_ms)),
    ]);
    table.add_row(vec![
        "像素转换阶段",
        &format!(
            "{stage_pixel_convert_ms} ms ({:.1}%)",
            stage_pct(stage_pixel_convert_ms)
        ),
    ]);
    table.add_row(vec![
        "编码预处理阶段",
        &format!(
            "{stage_encode_pre_ms} ms ({:.1}%)",
            stage_pct(stage_encode_pre_ms)
        ),
    ]);
    table.add_row(vec![
        "PNG优化阶段",
        &format!(
            "{stage_png_optimize_ms} ms ({:.1}%)",
            stage_pct(stage_png_optimize_ms)
        ),
    ]);
    table.add_row(vec![
        "编解码器阶段",
        &format!("{stage_codec_ms} ms ({:.1}%)", stage_pct(stage_codec_ms)),
    ]);
    table.add_row(vec![
        "写出阶段",
        &format!("{stage_write_ms} ms ({:.1}%)", stage_pct(stage_write_ms)),
    ]);
    if stage_svg_trace_ms > 0 || stage_svg_serialize_ms > 0 {
        let svg_total = stage_svg_trace_ms + stage_svg_serialize_ms;
        let svg_pct = |part: u64| {
            if svg_total > 0 {
                part as f64 / svg_total as f64 * 100.0
            } else {
                0.0
            }
        };
        table.add_row(vec!["SVG细分总耗时", &format!("{svg_total} ms")]);
        table.add_row(vec![
            "SVG追踪阶段",
            &format!(
                "{stage_svg_trace_ms} ms ({:.1}%)",
                svg_pct(stage_svg_trace_ms)
            ),
        ]);
        table.add_row(vec![
            "SVG序列化阶段",
            &format!(
                "{stage_svg_serialize_ms} ms ({:.1}%)",
                svg_pct(stage_svg_serialize_ms)
            ),
        ]);
        if stage_svg_trace_internal_ms > 0 {
            let vc_total = stage_svg_vc_to_color_ms
                + stage_svg_vc_keying_ms
                + stage_svg_vc_cluster_ms
                + stage_svg_vc_path_build_ms
                + stage_svg_vc_wrap_ms;
            let vc_pct = |part: u64| {
                if vc_total > 0 {
                    part as f64 / vc_total as f64 * 100.0
                } else {
                    0.0
                }
            };
            table.add_row(vec![
                "SVG内部追踪耗时",
                &format!("{stage_svg_trace_internal_ms} ms"),
            ]);
            table.add_row(vec![
                "VC转色图阶段",
                &format!(
                    "{stage_svg_vc_to_color_ms} ms ({:.1}%)",
                    vc_pct(stage_svg_vc_to_color_ms)
                ),
            ]);
            table.add_row(vec![
                "VC透明Key阶段",
                &format!(
                    "{stage_svg_vc_keying_ms} ms ({:.1}%)",
                    vc_pct(stage_svg_vc_keying_ms)
                ),
            ]);
            table.add_row(vec![
                "VC颜色聚类阶段",
                &format!(
                    "{stage_svg_vc_cluster_ms} ms ({:.1}%)",
                    vc_pct(stage_svg_vc_cluster_ms)
                ),
            ]);
            if stage_svg_vc_cluster_quantize_ms
                + stage_svg_vc_cluster_label_ms
                + stage_svg_vc_cluster_stats_ms
                + stage_svg_vc_cluster_merge_ms
                + stage_svg_vc_cluster_finalize_ms
                > 0
            {
                let vc_cluster_total = stage_svg_vc_cluster_quantize_ms
                    + stage_svg_vc_cluster_label_ms
                    + stage_svg_vc_cluster_stats_ms
                    + stage_svg_vc_cluster_merge_ms
                    + stage_svg_vc_cluster_finalize_ms;
                let vc_cluster_pct = |part: u64| {
                    if vc_cluster_total > 0 {
                        part as f64 / vc_cluster_total as f64 * 100.0
                    } else {
                        0.0
                    }
                };
                table.add_row(vec![
                    "VC聚类-量化阶段",
                    &format!(
                        "{stage_svg_vc_cluster_quantize_ms} ms ({:.1}%)",
                        vc_cluster_pct(stage_svg_vc_cluster_quantize_ms)
                    ),
                ]);
                table.add_row(vec![
                    "VC聚类-连通域阶段",
                    &format!(
                        "{stage_svg_vc_cluster_label_ms} ms ({:.1}%)",
                        vc_cluster_pct(stage_svg_vc_cluster_label_ms)
                    ),
                ]);
                table.add_row(vec![
                    "VC聚类-统计阶段",
                    &format!(
                        "{stage_svg_vc_cluster_stats_ms} ms ({:.1}%)",
                        vc_cluster_pct(stage_svg_vc_cluster_stats_ms)
                    ),
                ]);
                table.add_row(vec![
                    "VC聚类-小区合并阶段",
                    &format!(
                        "{stage_svg_vc_cluster_merge_ms} ms ({:.1}%)",
                        vc_cluster_pct(stage_svg_vc_cluster_merge_ms)
                    ),
                ]);
                table.add_row(vec![
                    "VC聚类-组件收敛阶段",
                    &format!(
                        "{stage_svg_vc_cluster_finalize_ms} ms ({:.1}%)",
                        vc_cluster_pct(stage_svg_vc_cluster_finalize_ms)
                    ),
                ]);
            }
            table.add_row(vec![
                "VC路径生成阶段",
                &format!(
                    "{stage_svg_vc_path_build_ms} ms ({:.1}%)",
                    vc_pct(stage_svg_vc_path_build_ms)
                ),
            ]);
            if stage_svg_vc_path_sort_ms
                + stage_svg_vc_path_trace_ms
                + stage_svg_vc_path_smooth_ms
                + stage_svg_vc_path_svg_emit_ms
                > 0
            {
                let vc_path_total = stage_svg_vc_path_sort_ms
                    + stage_svg_vc_path_trace_ms
                    + stage_svg_vc_path_smooth_ms
                    + stage_svg_vc_path_svg_emit_ms;
                let vc_path_pct = |part: u64| {
                    if vc_path_total > 0 {
                        part as f64 / vc_path_total as f64 * 100.0
                    } else {
                        0.0
                    }
                };
                table.add_row(vec![
                    "VC路径-排序阶段",
                    &format!(
                        "{stage_svg_vc_path_sort_ms} ms ({:.1}%)",
                        vc_path_pct(stage_svg_vc_path_sort_ms)
                    ),
                ]);
                table.add_row(vec![
                    "VC路径-轮廓提取阶段",
                    &format!(
                        "{stage_svg_vc_path_trace_ms} ms ({:.1}%)",
                        vc_path_pct(stage_svg_vc_path_trace_ms)
                    ),
                ]);
                table.add_row(vec![
                    "VC路径-平滑阶段",
                    &format!(
                        "{stage_svg_vc_path_smooth_ms} ms ({:.1}%)",
                        vc_path_pct(stage_svg_vc_path_smooth_ms)
                    ),
                ]);
                table.add_row(vec![
                    "VC路径-SVG拼接阶段",
                    &format!(
                        "{stage_svg_vc_path_svg_emit_ms} ms ({:.1}%)",
                        vc_path_pct(stage_svg_vc_path_svg_emit_ms)
                    ),
                ]);
                if stage_svg_vc_path_components_total > 0 {
                    let simplified_ratio = stage_svg_vc_path_components_simplified as f64
                        / stage_svg_vc_path_components_total as f64
                        * 100.0;
                    let smooth_ratio = stage_svg_vc_path_components_smoothed as f64
                        / stage_svg_vc_path_components_total as f64
                        * 100.0;
                    table.add_row(vec![
                        "VC路径-简化组件占比",
                        &format!(
                            "{}/{} ({simplified_ratio:.1}%)",
                            stage_svg_vc_path_components_simplified,
                            stage_svg_vc_path_components_total
                        ),
                    ]);
                    table.add_row(vec![
                        "VC路径-平滑组件占比",
                        &format!(
                            "{}/{} ({smooth_ratio:.1}%)",
                            stage_svg_vc_path_components_smoothed,
                            stage_svg_vc_path_components_total
                        ),
                    ]);
                }
            }
            table.add_row(vec![
                "VC封装输出阶段",
                &format!(
                    "{stage_svg_vc_wrap_ms} ms ({:.1}%)",
                    vc_pct(stage_svg_vc_wrap_ms)
                ),
            ]);
        }
    }
    table.add_row(vec!["平均阶段耗时", &format!("{stage_avg_ms:.1} ms")]);
    table.add_row(vec!["总耗时", &format!("{:.2}s", wall_ms as f64 / 1000.0)]);

    eprintln!("\n{table}");

    if !failed.is_empty() {
        eprintln!("\n失败详情:");
        for r in failed {
            eprintln!(
                "  {} -> {} - {}",
                r.input_path.display(),
                r.output_path.display(),
                r.error.as_deref().unwrap_or("未知错误")
            );
        }
    }
}
