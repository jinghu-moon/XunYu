#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import math
import statistics
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Dict, Iterable, List, Tuple

import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter


@dataclass
class CaseResult:
    tool: str
    case_id: str
    category: str
    input_rel: str
    input_file: str
    width: int
    height: int
    elapsed_ms: float
    output_bytes: int
    path_count: int
    psnr: float
    ssim: float
    mae: float
    render_ok: bool
    error: str
    svg_file: str
    raster_file: str


def composite_to_white_rgb(image: Image.Image) -> np.ndarray:
    rgba = image.convert("RGBA")
    arr = np.asarray(rgba).astype(np.float64)
    rgb = arr[:, :, :3]
    alpha = arr[:, :, 3:4] / 255.0
    out = rgb * alpha + 255.0 * (1.0 - alpha)
    return out


def compute_psnr(a: np.ndarray, b: np.ndarray) -> float:
    mse = float(np.mean((a - b) ** 2))
    if mse <= 1e-12:
        return float("inf")
    return 20.0 * math.log10(255.0) - 10.0 * math.log10(mse)


def compute_ssim_channel(x: np.ndarray, y: np.ndarray, sigma: float = 1.5) -> float:
    c1 = (0.01 * 255.0) ** 2
    c2 = (0.03 * 255.0) ** 2

    mu_x = gaussian_filter(x, sigma=sigma)
    mu_y = gaussian_filter(y, sigma=sigma)

    mu_x2 = mu_x * mu_x
    mu_y2 = mu_y * mu_y
    mu_xy = mu_x * mu_y

    sigma_x2 = gaussian_filter(x * x, sigma=sigma) - mu_x2
    sigma_y2 = gaussian_filter(y * y, sigma=sigma) - mu_y2
    sigma_xy = gaussian_filter(x * y, sigma=sigma) - mu_xy

    num = (2.0 * mu_xy + c1) * (2.0 * sigma_xy + c2)
    den = (mu_x2 + mu_y2 + c1) * (sigma_x2 + sigma_y2 + c2)
    ssim_map = num / np.maximum(den, 1e-12)
    return float(np.mean(ssim_map))


def compute_ssim_rgb(a: np.ndarray, b: np.ndarray) -> float:
    scores = [compute_ssim_channel(a[:, :, i], b[:, :, i]) for i in range(3)]
    return float(np.mean(scores))


def parse_float(row: Dict[str, str], key: str, default: float = 0.0) -> float:
    v = row.get(key, "")
    if v is None or v == "":
        return default
    try:
        return float(v)
    except ValueError:
        return default


def parse_int(row: Dict[str, str], key: str, default: int = 0) -> int:
    v = row.get(key, "")
    if v is None or v == "":
        return default
    try:
        return int(float(v))
    except ValueError:
        return default


def ensure_dir(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)


def render_svg(
    rsvg_convert: str,
    svg_file: Path,
    out_png: Path,
    width: int,
    height: int,
    timeout_sec: int,
) -> Tuple[bool, str]:
    cmd = [
        rsvg_convert,
        "-w",
        str(width),
        "-h",
        str(height),
        "-f",
        "png",
        "-o",
        str(out_png),
        str(svg_file),
    ]
    try:
        proc = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout_sec,
            check=False,
        )
    except Exception as exc:
        return False, f"render exception: {exc}"

    if proc.returncode != 0:
        err = (proc.stderr or proc.stdout or "").strip()
        return False, f"render failed code={proc.returncode}: {err}"
    if not out_png.exists():
        return False, "render failed: png not generated"
    return True, ""


def summarize_numeric(values: Iterable[float]) -> Tuple[float, float, float]:
    arr = []
    for v in values:
        fv = float(v)
        if not math.isfinite(fv):
            fv = 99.0
        arr.append(fv)
    if not arr:
        return 0.0, 0.0, 0.0
    mean_v = float(statistics.mean(arr))
    median_v = float(statistics.median(arr))
    std_v = float(statistics.pstdev(arr)) if len(arr) > 1 else 0.0
    return mean_v, median_v, std_v


def write_csv(path: Path, rows: List[Dict[str, object]], fieldnames: List[str]) -> None:
    with path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Evaluate SVG quality by re-rasterizing and computing PSNR/SSIM."
    )
    parser.add_argument("--results-csv", required=True, help="Path to benchmark results.csv")
    parser.add_argument("--dataset-root", required=True, help="Path to source image dataset root")
    parser.add_argument("--out-dir", default="", help="Output directory")
    parser.add_argument("--rsvg-convert", default="rsvg-convert", help="Path to rsvg-convert")
    parser.add_argument("--timeout-sec", type=int, default=60, help="Render timeout per file")
    parser.add_argument("--max-cases", type=int, default=0, help="Limit number of rows")
    args = parser.parse_args()

    results_csv = Path(args.results_csv).resolve()
    dataset_root = Path(args.dataset_root).resolve()
    if not results_csv.exists():
        print(f"results csv not found: {results_csv}", file=sys.stderr)
        return 2
    if not dataset_root.exists():
        print(f"dataset root not found: {dataset_root}", file=sys.stderr)
        return 2

    if args.out_dir:
        out_dir = Path(args.out_dir).resolve()
    else:
        stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        out_dir = results_csv.parent / f"quality-{stamp}"
    raster_dir = out_dir / "rasterized"
    ensure_dir(out_dir)
    ensure_dir(raster_dir)

    rows: List[Dict[str, str]] = []
    with results_csv.open("r", encoding="utf-8-sig", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            rows.append(row)

    filtered = [
        r
        for r in rows
        if (r.get("output_exists", "").lower() == "true")
        and int(float(r.get("exit_code", "1"))) == 0
        and r.get("output_file")
    ]
    filtered.sort(key=lambda r: (r.get("tool", ""), r.get("case_id", "")))
    if args.max_cases > 0:
        filtered = filtered[: args.max_cases]

    case_results: List[CaseResult] = []
    total = len(filtered)
    for idx, row in enumerate(filtered, start=1):
        tool = row.get("tool", "")
        case_id = row.get("case_id", "")
        category = row.get("category", "")
        input_rel = row.get("input_rel", "")
        input_file = row.get("input_file", "")
        svg_file = Path(row.get("output_file", "")).resolve()
        source_file = (dataset_root / input_rel).resolve()
        elapsed_ms = parse_float(row, "elapsed_ms")
        output_bytes = parse_int(row, "output_bytes")
        path_count = parse_int(row, "path_count")

        raster_file = raster_dir / tool / f"{case_id}.png"
        ensure_dir(raster_file.parent)

        if not source_file.exists():
            case_results.append(
                CaseResult(
                    tool=tool,
                    case_id=case_id,
                    category=category,
                    input_rel=input_rel,
                    input_file=input_file,
                    width=0,
                    height=0,
                    elapsed_ms=elapsed_ms,
                    output_bytes=output_bytes,
                    path_count=path_count,
                    psnr=0.0,
                    ssim=0.0,
                    mae=0.0,
                    render_ok=False,
                    error=f"source missing: {source_file}",
                    svg_file=str(svg_file),
                    raster_file=str(raster_file),
                )
            )
            print(f"[{idx}/{total}] {tool} {input_rel} -> source missing")
            continue

        if not svg_file.exists():
            case_results.append(
                CaseResult(
                    tool=tool,
                    case_id=case_id,
                    category=category,
                    input_rel=input_rel,
                    input_file=input_file,
                    width=0,
                    height=0,
                    elapsed_ms=elapsed_ms,
                    output_bytes=output_bytes,
                    path_count=path_count,
                    psnr=0.0,
                    ssim=0.0,
                    mae=0.0,
                    render_ok=False,
                    error=f"svg missing: {svg_file}",
                    svg_file=str(svg_file),
                    raster_file=str(raster_file),
                )
            )
            print(f"[{idx}/{total}] {tool} {input_rel} -> svg missing")
            continue

        try:
            with Image.open(source_file) as src:
                w, h = src.size
                source_rgb = composite_to_white_rgb(src)
        except Exception as exc:
            case_results.append(
                CaseResult(
                    tool=tool,
                    case_id=case_id,
                    category=category,
                    input_rel=input_rel,
                    input_file=input_file,
                    width=0,
                    height=0,
                    elapsed_ms=elapsed_ms,
                    output_bytes=output_bytes,
                    path_count=path_count,
                    psnr=0.0,
                    ssim=0.0,
                    mae=0.0,
                    render_ok=False,
                    error=f"source decode error: {exc}",
                    svg_file=str(svg_file),
                    raster_file=str(raster_file),
                )
            )
            print(f"[{idx}/{total}] {tool} {input_rel} -> source decode error")
            continue

        ok, err = render_svg(
            rsvg_convert=args.rsvg_convert,
            svg_file=svg_file,
            out_png=raster_file,
            width=w,
            height=h,
            timeout_sec=max(1, int(args.timeout_sec)),
        )
        if not ok:
            case_results.append(
                CaseResult(
                    tool=tool,
                    case_id=case_id,
                    category=category,
                    input_rel=input_rel,
                    input_file=input_file,
                    width=w,
                    height=h,
                    elapsed_ms=elapsed_ms,
                    output_bytes=output_bytes,
                    path_count=path_count,
                    psnr=0.0,
                    ssim=0.0,
                    mae=0.0,
                    render_ok=False,
                    error=err,
                    svg_file=str(svg_file),
                    raster_file=str(raster_file),
                )
            )
            print(f"[{idx}/{total}] {tool} {input_rel} -> render failed")
            continue

        try:
            with Image.open(raster_file) as rast:
                rendered_rgb = composite_to_white_rgb(rast)
        except Exception as exc:
            case_results.append(
                CaseResult(
                    tool=tool,
                    case_id=case_id,
                    category=category,
                    input_rel=input_rel,
                    input_file=input_file,
                    width=w,
                    height=h,
                    elapsed_ms=elapsed_ms,
                    output_bytes=output_bytes,
                    path_count=path_count,
                    psnr=0.0,
                    ssim=0.0,
                    mae=0.0,
                    render_ok=False,
                    error=f"raster decode error: {exc}",
                    svg_file=str(svg_file),
                    raster_file=str(raster_file),
                )
            )
            print(f"[{idx}/{total}] {tool} {input_rel} -> raster decode error")
            continue

        if rendered_rgb.shape != source_rgb.shape:
            case_results.append(
                CaseResult(
                    tool=tool,
                    case_id=case_id,
                    category=category,
                    input_rel=input_rel,
                    input_file=input_file,
                    width=w,
                    height=h,
                    elapsed_ms=elapsed_ms,
                    output_bytes=output_bytes,
                    path_count=path_count,
                    psnr=0.0,
                    ssim=0.0,
                    mae=0.0,
                    render_ok=False,
                    error=f"shape mismatch src={source_rgb.shape}, out={rendered_rgb.shape}",
                    svg_file=str(svg_file),
                    raster_file=str(raster_file),
                )
            )
            print(f"[{idx}/{total}] {tool} {input_rel} -> shape mismatch")
            continue

        psnr = compute_psnr(source_rgb, rendered_rgb)
        ssim = compute_ssim_rgb(source_rgb, rendered_rgb)
        mae = float(np.mean(np.abs(source_rgb - rendered_rgb)))

        case_results.append(
            CaseResult(
                tool=tool,
                case_id=case_id,
                category=category,
                input_rel=input_rel,
                input_file=input_file,
                width=w,
                height=h,
                elapsed_ms=elapsed_ms,
                output_bytes=output_bytes,
                path_count=path_count,
                psnr=psnr,
                ssim=ssim,
                mae=mae,
                render_ok=True,
                error="",
                svg_file=str(svg_file),
                raster_file=str(raster_file),
            )
        )
        print(
            f"[{idx}/{total}] {tool} {input_rel} -> "
            f"PSNR={psnr:.3f} SSIM={ssim:.5f} MAE={mae:.3f}"
        )

    detail_rows: List[Dict[str, object]] = []
    for r in case_results:
        detail_rows.append(
            {
                "tool": r.tool,
                "case_id": r.case_id,
                "category": r.category,
                "input_rel": r.input_rel,
                "input_file": r.input_file,
                "width": r.width,
                "height": r.height,
                "elapsed_ms": f"{r.elapsed_ms:.4f}",
                "output_bytes": r.output_bytes,
                "path_count": r.path_count,
                "psnr": f"{r.psnr:.8f}",
                "ssim": f"{r.ssim:.8f}",
                "mae": f"{r.mae:.8f}",
                "render_ok": str(r.render_ok),
                "error": r.error,
                "svg_file": r.svg_file,
                "raster_file": r.raster_file,
            }
        )

    detail_csv = out_dir / "quality.details.csv"
    write_csv(
        detail_csv,
        detail_rows,
        [
            "tool",
            "case_id",
            "category",
            "input_rel",
            "input_file",
            "width",
            "height",
            "elapsed_ms",
            "output_bytes",
            "path_count",
            "psnr",
            "ssim",
            "mae",
            "render_ok",
            "error",
            "svg_file",
            "raster_file",
        ],
    )

    ok_rows = [r for r in case_results if r.render_ok]
    by_tool: Dict[str, List[CaseResult]] = {}
    by_category_tool: Dict[Tuple[str, str], List[CaseResult]] = {}
    for r in ok_rows:
        by_tool.setdefault(r.tool, []).append(r)
        by_category_tool.setdefault((r.category, r.tool), []).append(r)

    tool_summary_rows: List[Dict[str, object]] = []
    for tool, items in sorted(by_tool.items()):
        psnr_mean, psnr_median, psnr_std = summarize_numeric(x.psnr for x in items)
        ssim_mean, ssim_median, ssim_std = summarize_numeric(x.ssim for x in items)
        mae_mean, mae_median, mae_std = summarize_numeric(x.mae for x in items)
        elapsed_mean, elapsed_median, elapsed_std = summarize_numeric(x.elapsed_ms for x in items)
        out_mean, out_median, out_std = summarize_numeric(x.output_bytes for x in items)
        paths_mean, paths_median, paths_std = summarize_numeric(x.path_count for x in items)
        tool_summary_rows.append(
            {
                "tool": tool,
                "n": len(items),
                "psnr_mean": f"{psnr_mean:.8f}",
                "psnr_median": f"{psnr_median:.8f}",
                "psnr_std": f"{psnr_std:.8f}",
                "ssim_mean": f"{ssim_mean:.8f}",
                "ssim_median": f"{ssim_median:.8f}",
                "ssim_std": f"{ssim_std:.8f}",
                "mae_mean": f"{mae_mean:.8f}",
                "mae_median": f"{mae_median:.8f}",
                "mae_std": f"{mae_std:.8f}",
                "elapsed_ms_mean": f"{elapsed_mean:.8f}",
                "elapsed_ms_median": f"{elapsed_median:.8f}",
                "elapsed_ms_std": f"{elapsed_std:.8f}",
                "output_bytes_mean": f"{out_mean:.8f}",
                "output_bytes_median": f"{out_median:.8f}",
                "output_bytes_std": f"{out_std:.8f}",
                "path_count_mean": f"{paths_mean:.8f}",
                "path_count_median": f"{paths_median:.8f}",
                "path_count_std": f"{paths_std:.8f}",
            }
        )

    tool_summary_csv = out_dir / "quality.summary.by_tool.csv"
    write_csv(
        tool_summary_csv,
        tool_summary_rows,
        [
            "tool",
            "n",
            "psnr_mean",
            "psnr_median",
            "psnr_std",
            "ssim_mean",
            "ssim_median",
            "ssim_std",
            "mae_mean",
            "mae_median",
            "mae_std",
            "elapsed_ms_mean",
            "elapsed_ms_median",
            "elapsed_ms_std",
            "output_bytes_mean",
            "output_bytes_median",
            "output_bytes_std",
            "path_count_mean",
            "path_count_median",
            "path_count_std",
        ],
    )

    category_rows: List[Dict[str, object]] = []
    for (category, tool), items in sorted(by_category_tool.items()):
        psnr_mean, _, _ = summarize_numeric(x.psnr for x in items)
        ssim_mean, _, _ = summarize_numeric(x.ssim for x in items)
        mae_mean, _, _ = summarize_numeric(x.mae for x in items)
        elapsed_mean, _, _ = summarize_numeric(x.elapsed_ms for x in items)
        out_mean, _, _ = summarize_numeric(x.output_bytes for x in items)
        paths_mean, _, _ = summarize_numeric(x.path_count for x in items)
        category_rows.append(
            {
                "category": category,
                "tool": tool,
                "n": len(items),
                "psnr_mean": f"{psnr_mean:.8f}",
                "ssim_mean": f"{ssim_mean:.8f}",
                "mae_mean": f"{mae_mean:.8f}",
                "elapsed_ms_mean": f"{elapsed_mean:.8f}",
                "output_bytes_mean": f"{out_mean:.8f}",
                "path_count_mean": f"{paths_mean:.8f}",
            }
        )

    category_csv = out_dir / "quality.summary.by_category.csv"
    write_csv(
        category_csv,
        category_rows,
        [
            "category",
            "tool",
            "n",
            "psnr_mean",
            "ssim_mean",
            "mae_mean",
            "elapsed_ms_mean",
            "output_bytes_mean",
            "path_count_mean",
        ],
    )

    report_lines: List[str] = []
    report_lines.append("# SVG Quality Benchmark Report")
    report_lines.append("")
    report_lines.append(f"- Generated at: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    report_lines.append(f"- Results CSV: `{results_csv}`")
    report_lines.append(f"- Dataset root: `{dataset_root}`")
    report_lines.append(f"- rsvg-convert: `{args.rsvg_convert}`")
    report_lines.append(f"- Total rows in results.csv: {len(rows)}")
    report_lines.append(f"- Quality evaluated rows: {len(filtered)}")
    report_lines.append(f"- Render success rows: {len(ok_rows)}")
    report_lines.append("")
    report_lines.append("## By Tool")
    report_lines.append("")
    report_lines.append("| tool | n | psnr_mean | ssim_mean | mae_mean | elapsed_ms_mean | output_bytes_mean | path_count_mean |")
    report_lines.append("|---|---:|---:|---:|---:|---:|---:|---:|")
    for row in tool_summary_rows:
        report_lines.append(
            "| {tool} | {n} | {psnr_mean} | {ssim_mean} | {mae_mean} | {elapsed_ms_mean} | {output_bytes_mean} | {path_count_mean} |".format(
                **row
            )
        )
    report_lines.append("")
    report_lines.append("## By Category")
    report_lines.append("")
    report_lines.append("| category | tool | n | psnr_mean | ssim_mean | mae_mean | elapsed_ms_mean | output_bytes_mean | path_count_mean |")
    report_lines.append("|---|---|---:|---:|---:|---:|---:|---:|---:|")
    for row in category_rows:
        report_lines.append(
            "| {category} | {tool} | {n} | {psnr_mean} | {ssim_mean} | {mae_mean} | {elapsed_ms_mean} | {output_bytes_mean} | {path_count_mean} |".format(
                **row
            )
        )
    report_lines.append("")
    report_lines.append("## Output Files")
    report_lines.append("")
    report_lines.append(f"- quality.details.csv: `{detail_csv.name}`")
    report_lines.append(f"- quality.summary.by_tool.csv: `{tool_summary_csv.name}`")
    report_lines.append(f"- quality.summary.by_category.csv: `{category_csv.name}`")
    report_lines.append("")

    report_md = out_dir / "quality.report.md"
    report_md.write_text("\n".join(report_lines), encoding="utf-8")

    print("")
    print(f"[done] out_dir: {out_dir}")
    print(f"[done] details: {detail_csv}")
    print(f"[done] by_tool: {tool_summary_csv}")
    print(f"[done] by_category: {category_csv}")
    print(f"[done] report: {report_md}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
