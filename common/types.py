from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List


@dataclass
class DispatchTask:
    intent: str
    host: Dict[str, Any] = field(default_factory=dict)
    target: Dict[str, Any] = field(default_factory=dict)
    context: Dict[str, Any] = field(default_factory=dict)


@dataclass
class AgentResponse:
    agent: str
    status: str
    artifacts: List[Dict[str, Any]] = field(default_factory=list)
    logs: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "agent": self.agent,
            "status": self.status,
            "artifacts": self.artifacts,
            "logs": self.logs,
        }
