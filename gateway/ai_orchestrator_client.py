from __future__ import annotations

import json
import socket
from pathlib import Path
from typing import Any


class AiOrchestratorClient:
    """Small Unix socket client used by gateway integration tests and tooling."""

    def __init__(self, socket_path: str) -> None:
        self.socket_path = socket_path

    def request(self, message: dict[str, Any]) -> dict[str, Any]:
        payload = json.dumps(message).encode("utf-8") + b"\n"

        sock_path = Path(self.socket_path)
        if not sock_path.exists():
            raise FileNotFoundError(f"AI orchestrator socket not found: {self.socket_path}")

        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
            sock.connect(self.socket_path)
            sock.sendall(payload)

            data = b""
            while not data.endswith(b"\n"):
                chunk = sock.recv(4096)
                if not chunk:
                    break
                data += chunk

        if not data:
            raise RuntimeError("AI orchestrator returned an empty response")

        return json.loads(data.decode("utf-8").strip())


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Send a single JSON message to the AI orchestrator socket.")
    parser.add_argument("socket_path", help="Path to the Unix socket.")
    parser.add_argument("message", help="JSON message to send.")
    args = parser.parse_args()

    client = AiOrchestratorClient(args.socket_path)
    response = client.request(json.loads(args.message))
    print(json.dumps(response))
