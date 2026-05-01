from __future__ import annotations

import json
from http.server import BaseHTTPRequestHandler, HTTPServer

from common.protocol import parse_task
from common.types import AgentResponse, DispatchTask
import config


def generate(task: DispatchTask) -> AgentResponse:
    return AgentResponse(
        agent="APM",
        status="ok",
        artifacts=[
            {"type": "sbom", "content": "Placeholder SPDX SBOM entries."},
            {"type": "manifest", "content": "Placeholder package manifest with dependency graph."},
        ],
        logs=["APM started manifest generation", "APM resolved placeholder dependencies"],
    )


def process(task: DispatchTask) -> AgentResponse:
    return generate(task)


class APMHandler(BaseHTTPRequestHandler):
    def do_POST(self) -> None:
        if self.path not in ("/generate", "/process"):
            self.send_error(404)
            return
        length = int(self.headers.get("Content-Length", "0"))
        payload = json.loads(self.rfile.read(length) or b"{}")
        try:
            task = parse_task(payload)
            response = process(task).to_dict()
            self.send_response(200)
        except Exception as exc:
            response = AgentResponse(agent="APM", status="error", artifacts=[], logs=[str(exc)]).to_dict()
            self.send_response(400)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(response).encode("utf-8"))


def run() -> None:
    HTTPServer(("0.0.0.0", config.APM_PORT), APMHandler).serve_forever()


if __name__ == "__main__":
    run()
