from __future__ import annotations

import json
from http.server import BaseHTTPRequestHandler, HTTPServer

from common.protocol import parse_task
from common.types import AgentResponse, DispatchTask
import config


def analyze(task: DispatchTask) -> AgentResponse:
    return AgentResponse(
        agent="ARX",
        status="ok",
        artifacts=[
            {"type": "crash_summary", "content": "Placeholder runtime crash signature analysis."},
            {"type": "stack_trace_hints", "content": "Check null dereference paths in network stack."},
        ],
        logs=["ARX ingested runtime diagnostics", "ARX produced crash analysis recommendations"],
    )


def process(task: DispatchTask) -> AgentResponse:
    return analyze(task)


class ARXHandler(BaseHTTPRequestHandler):
    def do_POST(self) -> None:
        if self.path not in ("/analyze", "/process"):
            self.send_error(404)
            return
        length = int(self.headers.get("Content-Length", "0"))
        payload = json.loads(self.rfile.read(length) or b"{}")
        try:
            task = parse_task(payload)
            response = process(task).to_dict()
            self.send_response(200)
        except Exception as exc:
            response = AgentResponse(agent="ARX", status="error", artifacts=[], logs=[str(exc)]).to_dict()
            self.send_response(400)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(response).encode("utf-8"))


def run() -> None:
    HTTPServer(("0.0.0.0", config.ARX_PORT), ARXHandler).serve_forever()


if __name__ == "__main__":
    run()
