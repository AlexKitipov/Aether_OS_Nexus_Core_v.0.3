from __future__ import annotations

import json
import threading
import time
import urllib.request

from gateway.server import run as run_gateway


def post_dispatch() -> str:
    payload = {
        "intent": "driver.generate",
        "target": {"vendor": "intel", "device": "i225-v"},
        "host": {"os": "freebsd", "release": "14.1"},
        "context": {},
    }
    req = urllib.request.Request(
        "http://127.0.0.1:5100/v1/dispatch",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json", "Authorization": "Bearer demo-token"},
        method="POST",
    )
    with urllib.request.urlopen(req) as response:
        return response.read().decode("utf-8")


if __name__ == "__main__":
    thread = threading.Thread(target=run_gateway, daemon=True)
    thread.start()
    time.sleep(0.3)
    print(post_dispatch())
