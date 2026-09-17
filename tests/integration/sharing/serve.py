"""Loopback-only fallback preview. No request logging or deployment."""
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[3]
SITE = ROOT/'.work/mc031/site'


class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(SITE), **kwargs)

    def log_message(self, *args):
        pass  # Never record shared paths or display names.

    def do_GET(self):
        path = urlsplit(self.path).path
        if path.startswith(('/j/', '/friend/')):
            self.path = '/index.html'
        elif path not in ['/', '/index.html', '/style.css', '/share.mjs', '/page.mjs', '/config.mjs',
                          '/.well-known/assetlinks.json', '/.well-known/apple-app-site-association']:
            self.send_error(404)
            return
        super().do_GET()

    def end_headers(self):
        self.send_header('Cache-Control', 'no-store')
        self.send_header('Referrer-Policy', 'no-referrer')
        self.send_header('X-Content-Type-Options', 'nosniff')
        super().end_headers()

    def guess_type(self, path):
        if path.endswith('apple-app-site-association'):
            return 'application/json'
        return super().guess_type(path)


if __name__ == '__main__':
    ThreadingHTTPServer(('127.0.0.1', 8765), Handler).serve_forever()
