use super::*;

pub(super) fn build_paths_with_official_clusters(
    clusters: &VcColorClusters,
    small_cluster_polygon_area: usize,
) -> String {
    let view = clusters.view();
    let mut paths = String::with_capacity(view.clusters_output.len() * 64);
    for &ci in view.clusters_output.iter().rev() {
        let cluster = view.get_cluster(ci);
        let simplify_mode = if cluster.area() <= small_cluster_polygon_area {
            PathSimplifyMode::Polygon
        } else {
            PathSimplifyMode::Spline
        };
        let compound = cluster.to_compound_path(
            &view,
            false,
            simplify_mode,
            60.0_f64.to_radians(),
            4.0,
            10,
            45.0_f64.to_radians(),
        );
        let color = cluster.residue_color();
        let (s, offset) = compound.to_svg_string(true, PointF64::default(), Some(3));
        let _ = writeln!(
            &mut paths,
            "<path d=\"{s}\" fill=\"{}\" transform=\"translate({},{})\"/>",
            color.to_hex_string(),
            offset.x,
            offset.y
        );
    }
    paths
}

pub(super) fn build_paths_with_xun_components(
    components: &mut [XunComponent],
    cfg: PathBuildConfig,
) -> (String, XunPathBuildStats) {
    let mut stats = XunPathBuildStats {
        components_total: components.len() as u64,
        ..XunPathBuildStats::default()
    };

    let sort_start = Instant::now();
    components.sort_by_key(|c| Reverse(c.area));
    stats.sort_ms = sort_start.elapsed().as_millis() as u64;

    let mut paths = String::with_capacity(components.len() * 64);
    let mut trace_duration = Duration::default();
    let mut smooth_duration = Duration::default();
    let mut emit_duration = Duration::default();
    for comp in components.iter() {
        let (simplify_mode, post_smooth) = match cfg.mode {
            XunPathMode::None => (PathSimplifyMode::None, false),
            XunPathMode::Smooth => (PathSimplifyMode::None, true),
            XunPathMode::HybridSmooth => (PathSimplifyMode::None, comp.area >= cfg.smooth_min_area),
            XunPathMode::HybridSpline => {
                if comp.area >= cfg.smooth_min_area {
                    (PathSimplifyMode::Spline, false)
                } else {
                    (PathSimplifyMode::None, false)
                }
            }
            XunPathMode::Polygon => (PathSimplifyMode::Polygon, false),
            XunPathMode::Spline => (PathSimplifyMode::Spline, false),
            XunPathMode::Adaptive => {
                if comp.area <= cfg.polygon_area {
                    (PathSimplifyMode::Polygon, false)
                } else {
                    (PathSimplifyMode::Spline, false)
                }
            }
        };
        if !matches!(simplify_mode, PathSimplifyMode::None) {
            stats.components_simplified = stats.components_simplified.saturating_add(1);
        }

        let trace_start = Instant::now();
        let mut compound = BinaryCluster::image_to_compound_path(
            &PointI32 {
                x: comp.origin_x as i32,
                y: comp.origin_y as i32,
            },
            &comp.mask,
            simplify_mode,
            cfg.corner_threshold_rad,
            cfg.segment_length,
            cfg.max_iterations,
            cfg.splice_threshold_rad,
        );
        trace_duration += trace_start.elapsed();

        if post_smooth {
            let smooth_start = Instant::now();
            compound = compound.smooth(
                cfg.corner_threshold_rad,
                XUN_OUTSET_RATIO_DEFAULT,
                cfg.segment_length,
            );
            smooth_duration += smooth_start.elapsed();
            stats.components_smoothed = stats.components_smoothed.saturating_add(1);
        }

        let emit_start = Instant::now();
        let (s, offset) = compound.to_svg_string(true, PointF64::default(), Some(3));
        let _ = writeln!(
            &mut paths,
            "<path d=\"{s}\" fill=\"{}\" transform=\"translate({},{})\"/>",
            comp.color.to_hex_string(),
            offset.x,
            offset.y
        );
        emit_duration += emit_start.elapsed();
    }
    stats.trace_ms = (trace_duration.as_micros() / 1_000) as u64;
    stats.smooth_ms = (smooth_duration.as_micros() / 1_000) as u64;
    stats.svg_emit_ms = (emit_duration.as_micros() / 1_000) as u64;
    (paths, stats)
}
