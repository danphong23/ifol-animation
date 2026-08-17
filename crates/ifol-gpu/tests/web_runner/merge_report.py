"""Merge one Desktop/WebGPU test result into the canonical per-TC report."""

import argparse
import hashlib
import json
import os
from pathlib import Path


def fnv1a64(data: bytes) -> str:
    value = 0xCBF29CE484222325
    for byte in data:
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{value:016x}"


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def uniform_check(raw: bytes, expected: list[int] | None) -> str:
    if not expected:
        return "KHÔNG ÁP DỤNG"
    pixel = bytes(expected)
    if len(raw) % len(pixel) != 0:
        return "KHÔNG ĐẠT: raw không căn theo RGBA8"
    return "ĐẠT" if raw == pixel * (len(raw) // len(pixel)) else "KHÔNG ĐẠT"


def content_check(raw: bytes, evaluation: dict) -> str:
    expected = evaluation.get("expected_rgba8")
    if expected:
        return uniform_check(raw, expected)
    if evaluation.get("expected_non_empty"):
        pixels = {raw[index:index + 4] for index in range(0, len(raw), 4)}
        return "ĐẠT" if len(pixels) > 1 else "KHÔNG ĐẠT: output đồng nhất ngoài dự kiến"
    return "KHÔNG ÁP DỤNG"


def format_ms(value) -> str:
    if value is None:
        return "CHƯA GHI NHẬN"
    value = float(value)
    if value < 1.0:
        return f"{value:.4f} ms ({value * 1000.0:.1f} µs)"
    return f"{value:.4f} ms"


def structure_metrics(raw: bytes, background: list[int] | None, width: int) -> tuple[int, tuple[int, int, int, int] | None, list[bool]]:
    if not background:
        return 0, None, []
    background_pixel = tuple(background)
    pixels = [tuple(raw[index:index + 4]) for index in range(0, len(raw), 4)]
    mask = [pixel != background_pixel for pixel in pixels]
    positions = [index for index, active in enumerate(mask) if active]
    if not positions:
        return 0, None, mask
    xs = [index % width for index in positions]
    ys = [index // width for index in positions]
    return len(positions), (min(xs), min(ys), max(xs), max(ys)), mask


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--desktop-meta", type=Path, required=True)
    parser.add_argument("--desktop-raw", type=Path, required=True)
    parser.add_argument("--desktop-image", type=Path, required=True)
    parser.add_argument("--web-meta", type=Path, required=True)
    parser.add_argument("--web-raw", type=Path, required=True)
    parser.add_argument("--web-image", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--vision-desktop", default="CHƯA ĐÁNH GIÁ")
    parser.add_argument("--vision-web", default="CHƯA ĐÁNH GIÁ")
    args = parser.parse_args()

    manifest_text = args.manifest.read_text(encoding="utf-8")
    manifest = json.loads(manifest_text)
    desktop = read_json(args.desktop_meta)
    web = read_json(args.web_meta)
    desktop_raw = args.desktop_raw.read_bytes()
    web_raw = args.web_raw.read_bytes()
    manifest_fingerprint = fnv1a64(manifest_text.encode("utf-8"))
    desktop_raw_fingerprint = fnv1a64(desktop_raw)
    web_raw_fingerprint = fnv1a64(web_raw)
    same_size = len(desktop_raw) == len(web_raw)
    different_bytes = sum(a != b for a, b in zip(desktop_raw, web_raw))
    different_bytes += abs(len(desktop_raw) - len(web_raw))
    comparable_pixels = min(len(desktop_raw), len(web_raw)) // 4
    different_pixels = 0
    max_pixel_delta = 0
    for index in range(comparable_pixels):
        desktop_pixel = desktop_raw[index * 4:index * 4 + 4]
        web_pixel = web_raw[index * 4:index * 4 + 4]
        if desktop_pixel != web_pixel:
            different_pixels += 1
            max_pixel_delta = max(
                max_pixel_delta,
                max(abs(a - b) for a, b in zip(desktop_pixel, web_pixel)),
            )
    graph_match = (
        desktop.get("manifest_fingerprint") == manifest_fingerprint
        and web.get("manifest_fingerprint") == manifest_fingerprint
    )
    exact_match = same_size and different_bytes == 0
    target = manifest["graph"]["target"]
    evaluation = manifest.get("evaluation", {})
    desktop_content = content_check(desktop_raw, evaluation)
    web_content = content_check(web_raw, evaluation)
    background = evaluation.get("expected_background_rgba8")
    target_width = target["width"]
    desktop_count, desktop_bbox, desktop_mask = structure_metrics(desktop_raw, background, target_width)
    web_count, web_bbox, web_mask = structure_metrics(web_raw, background, target_width)
    mask_diff = sum(a != b for a, b in zip(desktop_mask, web_mask)) if desktop_mask and web_mask else 0
    bbox_match = desktop_bbox == web_bbox
    mask_tolerance = evaluation.get("non_background_mask_diff_tolerance", 0)
    structure_match = bbox_match and mask_diff <= mask_tolerance
    structure_count_text = (
        f"{desktop_count} / {web_count}" if background else "KHÔNG ÁP DỤNG"
    )
    desktop_bbox_text = str(desktop_bbox) if background else "KHÔNG ÁP DỤNG"
    web_bbox_text = str(web_bbox) if background else "KHÔNG ÁP DỤNG"
    sha_desktop = hashlib.sha256(desktop_raw).hexdigest()
    sha_web = hashlib.sha256(web_raw).hexdigest()
    case_id = manifest["test_case"]
    title = manifest.get("title_vi", manifest["title"])
    description_vi = manifest.get("description_vi", manifest["description"])
    operations = manifest.get("graph", {}).get("operations", [])
    asset_names = sorted({operation.get("asset") for operation in operations if operation.get("asset")})
    pipeline_specs = manifest.get("graph", {}).get("pipelines", {})
    shader_names = sorted(
        {
            operation.get("shader")
            for operation in operations
            if operation.get("shader")
        }
        | {
            pipeline_specs.get(operation.get("pipeline"), {}).get("shader")
            for operation in operations
            if operation.get("pipeline")
            and pipeline_specs.get(operation.get("pipeline"), {}).get("shader")
        }
    )
    asset_text = ", ".join(f"`{asset}`" for asset in asset_names) or "KHÔNG KHAI BÁO"
    shader_text = ", ".join(f"`{shader}`" for shader in shader_names) or "KHÔNG KHAI BÁO"
    input_policy = (
        "Dùng PNG canonical để Desktop/WebGPU giải mã cùng một input byte-level."
        if asset_names and all(asset.startswith("canonical_") for asset in asset_names)
        else "Dùng asset theo manifest; chưa có chuẩn hóa input canonical riêng."
    )
    depth_text = json.dumps(manifest["graph"]["depth_stencil"], ensure_ascii=False) if "depth_stencil" in manifest.get("graph", {}) else "Không áp dụng"
    desktop_image_link = os.path.relpath(args.desktop_image, args.report.parent).replace(os.sep, "/")
    web_image_link = os.path.relpath(args.web_image, args.report.parent).replace(os.sep, "/")

    report = f"""# Báo cáo: {case_id} - {title}

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `{args.manifest.as_posix()}`
- **Graph fingerprint (FNV-1a):** `{manifest_fingerprint}`
- **Mô tả test case:** {description_vi}
- **Target:** `{target["width"]}x{target["height"]}`, `{target["format"]}`
- **Shader/WGSL:** {shader_text}
- **Asset/input:** {asset_text}
- **Chính sách input:** {input_policy}
- **Depth/stencil:** `{depth_text}`
- **Desktop/Web dùng cùng manifest fingerprint:** `{"ĐẠT" if graph_match else "KHÔNG ĐẠT"}`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `{format_ms(desktop.get("cold_render_time_ms"))}`
- **Thời gian render lần hai (warm/cache):** `{format_ms(desktop.get("warm_render_time_ms"))}`
- **Adapter/backend:** `{desktop.get("adapter_name", "không ghi nhận")}` / `{desktop.get("backend", "không ghi nhận")}`
- **Phạm vi timing:** `{desktop.get("timing_scope", "không ghi nhận")}`
- **Dữ liệu raw:** `{args.desktop_raw.as_posix()}`
- **Dấu vân tay raw (FNV-1a):** `{desktop_raw_fingerprint}`
- **SHA-256:** `{sha_desktop}`
- **Ảnh:** ![Desktop output]({desktop_image_link})
- **Đánh giá nội dung:** `{desktop_content}`
- **Đánh giá bằng vision:** {args.vision_desktop}

## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `{format_ms(web.get("cold_render_time_ms"))}`
- **Thời gian render lần hai (warm/cache):** `{format_ms(web.get("warm_render_time_ms"))}`
- **Adapter:** `{web.get("adapter_name", "không ghi nhận")}`
- **Phạm vi timing:** `{web.get("timing_scope", "không ghi nhận")}`
- **Dữ liệu raw:** `{args.web_raw.as_posix()}`
- **Dấu vân tay raw (FNV-1a):** `{web_raw_fingerprint}`
- **SHA-256:** `{sha_web}`
- **Ảnh:** ![WebGPU output]({web_image_link})
- **Đánh giá nội dung:** `{web_content}`
- **Đánh giá bằng vision:** {args.vision_web}

## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `{"ĐẠT" if graph_match else "KHÔNG ĐẠT"}` |
| Kích thước dữ liệu raw giống nhau | `{"ĐẠT" if same_size else "KHÔNG ĐẠT"}` |
| Byte raw giống tuyệt đối | `{"ĐẠT" if exact_match else "KHÔNG ĐẠT"}` |
| Số byte khác nhau | `{different_bytes}` |
| Số pixel khác nhau | `{different_pixels}` |
| Sai số kênh màu lớn nhất | `{max_pixel_delta}/255` |
| Khác biệt màu/presentation | `{"KHÔNG" if exact_match else "CÓ - cần theo dõi để đạt byte parity"}` |
| Số pixel non-background Desktop/Web | `{structure_count_text}` |
| Bounding box Desktop | `{desktop_bbox_text}` |
| Bounding box WebGPU | `{web_bbox_text}` |
| Bounding box non-background giống nhau | `{"ĐẠT" if bbox_match else "KHÔNG ĐẠT"}` |
| Số pixel mask khác nhau | `{mask_diff}` (ngưỡng `{mask_tolerance}`) |
| Parity cấu trúc không phụ thuộc màu | `{"ĐẠT" if structure_match else "KHÔNG ĐẠT"}` |
| Đúng mô tả test case | `{"ĐẠT" if desktop_content == "ĐẠT" and web_content == "ĐẠT" else "CẦN XEM LẠI"}` |

**Kết luận:** `{"ĐẠT - output giống tuyệt đối từng byte." if graph_match and exact_match else ("ĐẠT CÓ ĐIỀU KIỆN - graph và cấu trúc render giống; khác biệt còn lại thuộc pixel/màu và nằm trong ngưỡng đã khai báo." if graph_match and structure_match else "KHÔNG ĐẠT - cần điều tra khác biệt.")}`

## 5. Phân tích hiệu suất

Các giá trị trên đo thời gian thực thi graph, submit lệnh và chờ GPU hoàn tất;
không bao gồm khởi tạo device/pipeline hoặc readback. Vì vậy `cold` ở đây là
lần execute đầu sau khi resource/pipeline đã được tạo, không phải cold start
của toàn bộ ứng dụng. Giá trị dưới `1 ms` tương đương microsecond và cần được
đọc theo đơn vị đó khi phân tích.
"""
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(report, encoding="utf-8")


if __name__ == "__main__":
    main()
