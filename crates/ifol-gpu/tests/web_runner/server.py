import http.server
import socketserver
import json
import base64
import os
import sys

PORT = 8080
BASE_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.abspath(os.path.join(BASE_DIR, "..", "..", ".."))
GPU_CRATE_DIR = os.path.abspath(os.path.join(BASE_DIR, "..", ".."))
OUTPUT_DIR = os.path.join(GPU_CRATE_DIR, "tests", "outputs", "web")
SHADERS_DIR = os.path.join(GPU_CRATE_DIR, "tests", "shared_assets", "shaders")

os.makedirs(OUTPUT_DIR, exist_ok=True)

class WebGpuTestHandler(http.server.SimpleHTTPRequestHandler):
    def translate_path(self, path):
        # Clean path
        clean_path = path.split('?')[0].split('#')[0]
        if clean_path.startswith('/shaders/'):
            filename = clean_path[len('/shaders/'):]
            return os.path.join(SHADERS_DIR, filename)
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
