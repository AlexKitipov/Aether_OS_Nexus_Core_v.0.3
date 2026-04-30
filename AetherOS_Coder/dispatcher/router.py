"""Central dispatcher that routes tasks to AetherOS agents by intent."""

from __future__ import annotations

from typing import Any

from AetherOS_Coder.ADI import agent as ADI
from AetherOS_Coder.APM import agent as APM
from AetherOS_Coder.ARX import agent as ARX


def _log_dispatch(intent: str, routed_agent: str) -> dict[str, str]:
    """Create a standard log entry for routed tasks."""
    return {
        "event": "dispatch",
        "intent": intent,
        "agent": routed_agent,
    }


def dispatch(task: dict[str, Any]) -> dict[str, Any]:
    """Route task to the appropriate agent based on intent."""
    intent = task.get("intent", "")

    if intent.startswith("driver."):
        result = ADI.process(task)
        result.setdefault("logs", []).append(_log_dispatch(intent, "ADI"))
        return result

    if intent.startswith("runtime."):
        result = ARX.process(task)
        result.setdefault("logs", []).append(_log_dispatch(intent, "ARX"))
        return result

    if intent.startswith("package."):
        result = APM.process(task)
        result.setdefault("logs", []).append(_log_dispatch(intent, "APM"))
        return result

    return {
        "agent": "dispatcher",
        "status": "error",
        "artifacts": [],
        "logs": [_log_dispatch(intent, "unknown")],
        "message": "Unknown intent",
    }
