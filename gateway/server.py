from __future__ import annotations

import json
import os
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, HTTPServer
from typing import Callable

from agents.adi.server import process as adi_process
from agents.apm.server import process as apm_process
from agents.arx.server import process as arx_process
from common.protocol import parse_task
from common.types import AgentResponse, DispatchTask
import config


def validate_bearer_token(auth_header: str | None) -> bool:
    if not auth_header or not auth_header.startswith("Bearer "):
        return False
    token = auth_header.split(" ", 1)[1].strip()
    return bool(token)


def apply_rbac_placeholder(task: DispatchTask) -> bool:
    return True


def api_key_hook_placeholder(headers: dict) -> bool:
    return True


def select_agent(intent: str) -> Callable[[DispatchTask], AgentResponse]:
    if intent.startswith("driver."):
        return adi_process
    if intent.startswith("runtime."):
        return arx_process
    if intent.startswith("package."):
        return apm_process
    raise ValueError("Unsupported intent")


def _coder_dispatch_url() -> str | None:
    base = os.getenv("AETHEROS_CODER_BASE_URL", "").strip().rstrip("/")
    if not base:
        return None
    return f"{base}/v1/dispatch"


def dispatch_to_aetheros_coder(task: DispatchTask, auth_header: str | None) -> dict | None:
    url = _coder_dispatch_url()
    if not url:
        return None

    headers = {"Content-Type": "application/json"}
    if auth_header:
        headers["Authorization"] = auth_header

    payload = {
        "intent": task.intent,
        "host": task.host,
        "target": task.target,
        "context": task.context,
    }

    req = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        headers=headers,
        method="POST",
    )

    with urllib.request.urlopen(req, timeout=float(os.getenv("AETHEROS_CODER_TIMEOUT_SEC", "10"))) as response:
        raw = response.read().decode("utf-8")

    lines = [line for line in raw.splitlines() if line.strip()]
    if not lines:
        raise RuntimeError("AetherOS Coder returned empty response")

    return json.loads(lines[-1])


class GatewayHandler(BaseHTTPRequestHandler):
    def do_POST(self) -> None:
        if self.path != "/v1/dispatch":
            self.send_error(404)
            return

        auth_header = self.headers.get("Authorization")
        if not validate_bearer_token(auth_header):
            self.send_response(401)
            self.end_headers()
            return

        length = int(self.headers.get("Content-Length", "0"))
        payload = json.loads(self.rfile.read(length) or b"{}")

        self.send_response(200)
        self.send_header("Content-Type", "application/x-ndjson")
        self.end_headers()

        try:
            task = parse_task(payload)
            if not apply_rbac_placeholder(task) or not api_key_hook_placeholder(dict(self.headers)):
                raise PermissionError("Authorization policy denied request")

            self.wfile.write(json.dumps({"logs": [f"Routing intent: {task.intent}"]}).encode() + b"\n")

            external_result = dispatch_to_aetheros_coder(task, auth_header)
            if external_result is not None:
                self.wfile.write(json.dumps(external_result).encode() + b"\n")
                return

            handler = select_agent(task.intent)
            result = handler(task).to_dict()
            self.wfile.write(json.dumps(result).encode() + b"\n")
        except urllib.error.HTTPError as exc:
            error = AgentResponse(
                agent="GATEWAY",
                status="error",
                artifacts=[],
                logs=[f"AetherOS Coder HTTP error {exc.code}: {exc.reason}"],
            ).to_dict()
            self.wfile.write(json.dumps(error).encode() + b"\n")
        except Exception as exc:
            error = AgentResponse(agent="GATEWAY", status="error", artifacts=[], logs=[str(exc)]).to_dict()
            self.wfile.write(json.dumps(error).encode() + b"\n")


def run() -> None:
    HTTPServer(("0.0.0.0", config.GATEWAY_PORT), GatewayHandler).serve_forever()


if __name__ == "__main__":
    run()
