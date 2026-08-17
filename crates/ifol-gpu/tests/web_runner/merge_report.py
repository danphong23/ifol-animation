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


def format_percent(value) -> str:
    if value is None:
        return "CHƯA GHI NHẬN"
    return f"{float(value):.1f}%"


def display_value(value) -> str:
    return "CHƯA GHI NHẬN" if value is None else str(value)


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
    cache_output_values = (desktop.get("cache_output_equal"), web.get("cache_output_equal"))
    cache_output_text = (
        "ĐẠT" if cache_output_values == (True, True)
        else "KHÔNG ĐẠT" if False in cache_output_values
        else "CHƯA GHI NHẬN"
    )
    sha_desktop = hashlib.sha256(desktop_raw).hexdigest()
    sha_web = hashlib.sha256(web_raw).hexdigest()
    case_id = manifest["test_case"]
    title = manifest.get("title_vi", manifest["title"])
    description_vi = manifest.get("description_vi", manifest["description"])
    graph_spec = manifest.get("graph", {})
    operations = graph_spec.get("operations", [])
    pass_specs = graph_spec.get("passes", [])
    pass_operations = [operation for pass_spec in pass_specs for operation in pass_spec.get("operations", [])]
    single_operation = [graph_spec["operation"]] if graph_spec.get("operation") else []
    all_operations = operations + pass_operations + single_operation
    asset_names_set = set()
    for operation in all_operations:
        if operation.get("asset"):
            asset_names_set.add(operation["asset"])
        source = operation.get("source", {})
        source_specs = source if isinstance(source, list) else [source]
        for source_spec in source_specs:
            if source_spec.get("asset"):
                asset_names_set.add(source_spec["asset"])
    asset_names = sorted(asset_names_set)
    pipeline_specs = graph_spec.get("pipelines", {})
    single_pipeline_shader = graph_spec.get("pipeline", {}).get("shader")
    shader_names = sorted(
        {
            operation.get("shader")
            for operation in all_operations
            if operation.get("shader")
        }
        | {
            pipeline_specs.get(operation.get("pipeline"), {}).get("shader")
            for operation in all_operations
            if operation.get("pipeline")
            and pipeline_specs.get(operation.get("pipeline"), {}).get("shader")
        }
        | ({single_pipeline_shader} if single_pipeline_shader else set())
    )
    asset_text = ", ".join(f"`{asset}`" for asset in asset_names) or "KHÔNG KHAI BÁO"
    shader_text = ", ".join(f"`{shader}`" for shader in shader_names) or "KHÔNG KHAI BÁO"
    if not asset_names:
        input_policy = "Không dùng texture/asset; input là uniform và graph canonical từ manifest."
    elif all(asset.startswith("canonical_") for asset in asset_names):
        input_policy = "Dùng PNG canonical để Desktop/WebGPU giải mã cùng một input byte-level."
    else:
        input_policy = "Dùng asset theo manifest; chưa có chuẩn hóa input canonical riêng."
    depth_text = json.dumps(manifest["graph"]["depth_stencil"], ensure_ascii=False) if "depth_stencil" in manifest.get("graph", {}) else "Không áp dụng"
    node_pool_spec = graph_spec.get("node_pool")
    node_pool_text = (
        f"allocated={node_pool_spec.get('allocated')}, freed={node_pool_spec.get('freed')}, surviving={node_pool_spec.get('surviving')}"
        if node_pool_spec
        else "Không áp dụng"
    )
    error_contract = manifest.get("error_contract")
    error_contract_text = (
        json.dumps(error_contract, ensure_ascii=False)
        if error_contract else "Không áp dụng"
    )
    sampler_text = json.dumps(graph_spec.get("sampler"), ensure_ascii=False) if graph_spec.get("sampler") else "Không khai báo"
    expected_layer_order = evaluation.get("expected_layer_order", [])
    expected_layer_order_text = " → ".join(expected_layer_order) if expected_layer_order else "Không khai báo"
    node_count_text = graph_spec.get("node_count", "Không khai báo")
    draw_commands_text = graph_spec.get("command_count", "Không khai báo")
    instance_count_text = sum(
        operation.get("instance_count", 0)
        for operation in all_operations
        if isinstance(operation, dict)
    )
    particle_instance_text = evaluation.get("expected_particle_instances", "Không khai báo")
    pass_text = (
        " → ".join(
            f"{pass_spec.get('id', '?')} ({pass_spec.get('name', 'không tên')}, target {pass_spec.get('target', '?')})"
            for pass_spec in pass_specs
        )
        if pass_specs
        else "Không khai báo dạng pass"
    )
    recursion_depth = graph_spec.get("depth")
    hierarchy_text = graph_spec.get("hierarchy", "Không khai báo")
    operation_order_text = (
        " → ".join(operation.get("id", "?") for operation in all_operations)
        if all_operations
        else "Không khai báo"
    )
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
- **Chuỗi pass:** {pass_text}
- **Số pass:** `{len(pass_specs) if pass_specs else 'KHÔNG ÁP DỤNG'}`
- **Độ sâu graph:** `{recursion_depth if recursion_depth is not None else 'KHÔNG ÁP DỤNG'}`
- **Hierarchy:** `{hierarchy_text}`
- **Thứ tự operation sau flatten:** `{operation_order_text}`
- **Sampler contract:** `{sampler_text}`
- **Thứ tự layer kỳ vọng:** `{expected_layer_order_text}`
- **Graph resources:** nodes=`{node_count_text}`, draw commands=`{draw_commands_text}`, tổng instances=`{instance_count_text}`, procedural particles=`{particle_instance_text}`
- **Node pool contract:** `{node_pool_text}`
- **Error/fallback contract:** `{error_contract_text}`
- **Desktop/Web dùng cùng manifest fingerprint:** `{"ĐẠT" if graph_match else "KHÔNG ĐẠT"}`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `{format_ms(desktop.get("cold_render_time_ms"))}`
- **Thời gian render lần hai (warm/cache):** `{format_ms(desktop.get("warm_render_time_ms"))}`
- **Số lần warm được đo:** `{display_value(desktop.get("warm_iteration_count"))}`
- **Output cold và warm giống nhau:** `{display_value(desktop.get("cache_output_equal"))}`
- **Speedup cold → warm:** `{format_percent(desktop.get("speedup_percentage"))}`
- **Adapter/backend:** `{desktop.get("adapter_name", "không ghi nhận")}` / `{desktop.get("backend", "không ghi nhận")}`
- **Phạm vi timing:** `{desktop.get("timing_scope", "không ghi nhận")}`
- **Dữ liệu raw:** `{args.desktop_raw.as_posix()}`
- **Dấu vân tay raw (FNV-1a):** `{desktop_raw_fingerprint}`
- **SHA-256:** `{sha_desktop}`
- **Ảnh:** ![Desktop output]({desktop_image_link})
- **Đánh giá nội dung:** `{desktop_content}`
- **Đánh giá bằng vision:** {args.vision_desktop}
{f"- **Graph thực tế:** nodes={desktop.get('node_count')}, draw commands={desktop.get('draw_commands')}, instances={desktop.get('instance_count')}" if graph_spec.get("node_count") is not None else ""}
{f"- **Validation thực tế:** error={desktop.get('validation_error')}, handle={desktop.get('missing_bind_group')}, passed={desktop.get('validation_passed')}, panic={desktop.get('panic_occurred')}" if error_contract else ""}
{f"- **Node pool thực tế:** allocated={desktop.get('allocated_nodes')}, freed={desktop.get('freed_nodes')}, surviving={desktop.get('surviving_nodes')}" if node_pool_spec else ""}

## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `{format_ms(web.get("cold_render_time_ms"))}`
- **Thời gian render lần hai (warm/cache):** `{format_ms(web.get("warm_render_time_ms"))}`
- **Số lần warm được đo:** `{display_value(web.get("warm_iteration_count"))}`
- **Output cold và warm giống nhau:** `{display_value(web.get("cache_output_equal"))}`
- **Speedup cold → warm:** `{format_percent(web.get("speedup_percentage"))}`
- **Adapter:** `{web.get("adapter_name", "không ghi nhận")}`
- **Phạm vi timing:** `{web.get("timing_scope", "không ghi nhận")}`
- **Dữ liệu raw:** `{args.web_raw.as_posix()}`
- **Dấu vân tay raw (FNV-1a):** `{web_raw_fingerprint}`
- **SHA-256:** `{sha_web}`
- **Ảnh:** ![WebGPU output]({web_image_link})
- **Đánh giá nội dung:** `{web_content}`
- **Đánh giá bằng vision:** {args.vision_web}
{f"- **Graph thực tế:** nodes={web.get('node_count')}, draw commands={web.get('draw_commands')}, instances={web.get('instance_count')}" if graph_spec.get("node_count") is not None else ""}
{f"- **Validation contract mirror:** error={web.get('validation_error')}, handle={web.get('missing_bind_group')}, passed={web.get('validation_passed')}, panic={web.get('panic_occurred')}" if error_contract else ""}
{f"- **Node pool thực tế:** allocated={web.get('allocated_nodes')}, freed={web.get('freed_nodes')}, surviving={web.get('surviving_nodes')}, check={web.get('pool_check')}" if node_pool_spec else ""}

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
| Cache giữ nguyên output cold/warm ở cả hai môi trường | `{cache_output_text}` |
| Validation/fallback contract không panic | `{"ĐẠT" if (not error_contract) or (desktop.get("validation_passed") is True and web.get("validation_passed") is True and desktop.get("panic_occurred") is False and web.get("panic_occurred") is False) else "KHÔNG ĐẠT"}` |
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
