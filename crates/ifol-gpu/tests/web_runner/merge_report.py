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
        return "Not applicable"
    pixel = bytes(expected)
    if len(raw) % len(pixel) != 0:
        return "FAIL: raw output is not RGBA8-aligned"
    return "PASS" if raw == pixel * (len(raw) // len(pixel)) else "FAIL"


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
    graph_match = (
        desktop.get("manifest_fingerprint") == manifest_fingerprint
        and web.get("manifest_fingerprint") == manifest_fingerprint
    )
    exact_match = same_size and different_bytes == 0
    expected = manifest.get("evaluation", {}).get("expected_rgba8")
    desktop_content = uniform_check(desktop_raw, expected)
    web_content = uniform_check(web_raw, expected)
    sha_desktop = hashlib.sha256(desktop_raw).hexdigest()
    sha_web = hashlib.sha256(web_raw).hexdigest()
    target = manifest["graph"]["target"]
    case_id = manifest["test_case"]
    title = manifest["title"]
    desktop_image_link = os.path.relpath(args.desktop_image, args.report.parent).replace(os.sep, "/")
    web_image_link = os.path.relpath(args.web_image, args.report.parent).replace(os.sep, "/")

    report = f"""# Báo cáo: {case_id} - {title}

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và graph contract.

## 1. Mô tả và graph dùng chung

- **Manifest:** `{args.manifest.as_posix()}`
- **Graph fingerprint (FNV-1a):** `{manifest_fingerprint}`
- **Mô tả test case:** {manifest["description"]}
- **Target:** `{target["width"]}x{target["height"]}`, `{target["format"]}`
- **Desktop/Web dùng cùng manifest fingerprint:** `{"PASS" if graph_match else "FAIL"}`

## 2. Desktop

- **Cold render:** `{desktop.get("cold_render_time_ms")} ms`
- **Warm/cached render:** `{desktop.get("warm_render_time_ms")} ms`
- **Raw output:** `{args.desktop_raw.as_posix()}`
- **Raw fingerprint (FNV-1a):** `{desktop_raw_fingerprint}`
- **SHA-256:** `{sha_desktop}`
- **Ảnh:** ![Desktop output]({desktop_image_link})
- **Đánh giá nội dung:** `{desktop_content}`

## 3. WebGPU

- **Cold render:** `{web.get("cold_render_time_ms")} ms`
- **Warm/cached render:** `{web.get("warm_render_time_ms")} ms`
- **Raw output:** `{args.web_raw.as_posix()}`
- **Raw fingerprint (FNV-1a):** `{web_raw_fingerprint}`
- **SHA-256:** `{sha_web}`
- **Ảnh:** ![WebGPU output]({web_image_link})
- **Đánh giá nội dung:** `{web_content}`

## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `{"PASS" if graph_match else "FAIL"}` |
| Kích thước raw giống nhau | `{"PASS" if same_size else "FAIL"}` |
| Raw bytes giống tuyệt đối | `{"PASS" if exact_match else "FAIL"}` |
| Số byte khác nhau | `{different_bytes}` |
| Đúng mô tả test case | `{"PASS" if desktop_content == "PASS" and web_content == "PASS" else "REVIEW"}` |

**Kết luận:** `{"PASS - Desktop và WebGPU cho output canonical giống nhau từng byte." if graph_match and exact_match else "FAIL - cần điều tra khác biệt."}`
"""
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(report, encoding="utf-8")


if __name__ == "__main__":
    main()
