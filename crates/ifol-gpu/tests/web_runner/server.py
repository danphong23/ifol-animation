import http.server
import socketserver
import json
import base64
import hashlib
import os
import struct
import sys
import zlib

PORT = 8080
BASE_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.abspath(os.path.join(BASE_DIR, "..", "..", ".."))
GPU_CRATE_DIR = os.path.abspath(os.path.join(BASE_DIR, "..", ".."))
OUTPUT_DIR = os.path.join(GPU_CRATE_DIR, "tests", "outputs", "web")
SHADERS_DIR = os.path.join(GPU_CRATE_DIR, "tests", "shared_assets", "shaders")
MANIFESTS_DIR = os.path.join(GPU_CRATE_DIR, "tests", "shared_assets", "manifests")
TEXTURES_DIR = os.path.join(GPU_CRATE_DIR, "tests", "shared_assets", "textures")

os.makedirs(OUTPUT_DIR, exist_ok=True)


def write_rgba8_png(path, width, height, raw_bytes):
    row_size = width * 4
    if len(raw_bytes) != row_size * height:
        raise ValueError(f'RGBA8 payload size mismatch: {len(raw_bytes)} != {row_size * height}')

    def chunk(kind, payload):
        return (
            struct.pack('>I', len(payload))
            + kind
            + payload
            + struct.pack('>I', zlib.crc32(kind + payload) & 0xffffffff)
        )

    scanlines = b''.join(b'\x00' + raw_bytes[row * row_size:(row + 1) * row_size] for row in range(height))
    png = (
        b'\x89PNG\r\n\x1a\n'
        + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
        + chunk(b'IDAT', zlib.compress(scanlines, level=6))
        + chunk(b'IEND', b'')
    )
    with open(path, 'wb') as f:
        f.write(png)

class WebGpuTestHandler(http.server.SimpleHTTPRequestHandler):
    def translate_path(self, path):
        # Clean path
        clean_path = path.split('?')[0].split('#')[0]
        if clean_path.startswith('/shaders/'):
            filename = clean_path[len('/shaders/'):]
            return os.path.join(SHADERS_DIR, filename)
        elif clean_path.startswith('/manifests/'):
            filename = clean_path[len('/manifests/'):]
            return os.path.join(MANIFESTS_DIR, filename)
        elif clean_path.startswith('/textures/'):
            filename = clean_path[len('/textures/'):]
            return os.path.join(TEXTURES_DIR, filename)
        elif clean_path == '/' or clean_path == '/index.html':
            return os.path.join(BASE_DIR, "index.html")
        elif clean_path == '/web_runner.js':
            return os.path.join(BASE_DIR, "web_runner.js")
        return super().translate_path(path)

    def do_POST(self):
        if self.path == '/save_output':
            content_length = int(self.headers.get('Content-Length', 0))
            body = self.rfile.read(content_length)
            try:
                payload = json.loads(body.decode('utf-8'))
                filename = payload.get('name', 'output.png')
                img_data_url = payload.get('image', '')
                
                if ',' in img_data_url:
                    header, base64_str = img_data_url.split(',', 1)
                else:
                    base64_str = img_data_url

                img_bytes = base64.b64decode(base64_str)
                out_path = os.path.join(OUTPUT_DIR, filename)
                with open(out_path, 'wb') as f:
                    f.write(img_bytes)

                print(f"[Server] Saved WebGPU output: {out_path} ({len(img_bytes)} bytes)")

                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.send_header('Access-Control-Allow-Origin', '*')
                self.end_headers()
                self.wfile.write(json.dumps({'status': 'ok', 'saved': filename}).encode('utf-8'))
            except Exception as e:
                print(f"[Server Error] {e}")
                self.send_response(500)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps({'status': 'error', 'message': str(e)}).encode('utf-8'))
        elif self.path == '/save_raw':
            content_length = int(self.headers.get('Content-Length', 0))
            body = self.rfile.read(content_length)
            try:
                payload = json.loads(body.decode('utf-8'))
                filename = payload.get('name', 'output')
                raw_bytes = base64.b64decode(payload.get('bytes', ''))
                raw_path = os.path.join(OUTPUT_DIR, filename + '.bin')
                metadata_path = os.path.join(OUTPUT_DIR, filename + '.json')
                with open(raw_path, 'wb') as f:
                    f.write(raw_bytes)
                image_name = payload.get('image_name')
                if image_name:
                    image_path = os.path.join(OUTPUT_DIR, os.path.basename(image_name))
                    write_rgba8_png(image_path, int(payload['width']), int(payload['height']), raw_bytes)
                with open(metadata_path, 'w', encoding='utf-8') as f:
                    json.dump({
                        'width': payload.get('width'),
                        'height': payload.get('height'),
                        'format': payload.get('format'),
                        'render_time_ms': payload.get('render_time_ms'),
                        'cold_render_time_ms': payload.get('cold_render_time_ms'),
                        'warm_render_time_ms': payload.get('warm_render_time_ms'),
                        'warm_iteration_count': payload.get('warm_iteration_count'),
                        'speedup_percentage': payload.get('speedup_percentage'),
                        'cache_output_equal': payload.get('cache_output_equal'),
                        'validation_error': payload.get('validation_error'),
                        'missing_bind_group': payload.get('missing_bind_group'),
                        'validation_passed': payload.get('validation_passed'),
                        'panic_occurred': payload.get('panic_occurred'),
                        'fallback_color': payload.get('fallback_color'),
                        'manifest': payload.get('manifest'),
                        'manifest_fingerprint': payload.get('manifest_fingerprint'),
                        'adapter_name': payload.get('adapter_name'),
                        'timing_scope': payload.get('timing_scope'),
                        'isolation_scope': payload.get('isolation_scope'),
                        'allocated_nodes': payload.get('allocated_nodes'),
                        'freed_nodes': payload.get('freed_nodes'),
                        'surviving_nodes': payload.get('surviving_nodes'),
                        'pool_check': payload.get('pool_check'),
                        'recursion_depth': payload.get('recursion_depth'),
                        'flattened_operations': payload.get('flattened_operations'),
                        'node_count': payload.get('node_count'),
                        'draw_commands': payload.get('draw_commands'),
                        'instance_count': payload.get('instance_count'),
                        'pass_count': payload.get('pass_count'),
                        'viewport_count': payload.get('viewport_count'),
                        'byte_length': len(raw_bytes),
                        'sha256': hashlib.sha256(raw_bytes).hexdigest(),
                    }, f, indent=2)
                print(f"[Server] Saved raw WebGPU output: {raw_path} ({len(raw_bytes)} bytes)")
                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.send_header('Access-Control-Allow-Origin', '*')
                self.end_headers()
                self.wfile.write(json.dumps({'status': 'ok', 'saved': filename}).encode('utf-8'))
            except Exception as e:
                print(f"[Server Error] {e}")
                self.send_response(500)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps({'status': 'error', 'message': str(e)}).encode('utf-8'))
        else:
            self.send_response(404)
            self.end_headers()

    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        self.end_headers()

def run_server():
    socketserver.TCPServer.allow_reuse_address = True
    with socketserver.TCPServer(("", PORT), WebGpuTestHandler) as httpd:
        print(f"=== ifol-gpu WebGPU Test Server running on http://localhost:{PORT} ===")
        print(f"Outputs will be saved to: {OUTPUT_DIR}")
        sys.stdout.flush()
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nShutting down server.")

if __name__ == "__main__":
    run_server()
