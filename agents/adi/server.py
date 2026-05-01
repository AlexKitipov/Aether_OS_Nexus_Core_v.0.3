from __future__ import annotations

import json
from http.server import BaseHTTPRequestHandler, HTTPServer

from common.protocol import parse_task
from common.types import AgentResponse, DispatchTask
import config


def process(task: DispatchTask) -> AgentResponse:
    target = task.target
    host = task.host
    return AgentResponse(
        agent="ADI",
        status="ok",
        artifacts=[
            {
                "type": "driver_patch",
                "name": f"{target.get('vendor', 'generic')}_{target.get('device', 'device')}.patch",
                "content": "// placeholder driver adaptation patch",
            },
            {
                "type": "build_instructions",
                "content": f"Build on {host.get('os', 'linux')} {host.get('release', 'latest')} with make drivers",
            },
            {"type": "smoke_test_report", "content": "All placeholder smoke tests passed."},
        ],
        logs=["ADI started driver generation", "ADI completed adaptation workflow"],
    )


class ADIHandler(BaseHTTPRequestHandler):
    def do_POST(self) -> None:
        if self.path != "/process":
            self.send_error(404)
            return
        length = int(self.headers.get("Content-Length", "0"))
        payload = json.loads(self.rfile.read(length) or b"{}")
        try:
            task = parse_task(payload)
            response = process(task).to_dict()
            self.send_response(200)
        except Exception as exc:
            response = AgentResponse(agent="ADI", status="error", artifacts=[], logs=[str(exc)]).to_dict()
            self.send_response(400)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(response).encode("utf-8"))


def run() -> None:
    HTTPServer(("0.0.0.0", config.ADI_PORT), ADIHandler).serve_forever()


if __name__ == "__main__":
    run()
