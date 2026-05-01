from __future__ import annotations

from typing import Any, Dict

from common.types import DispatchTask


VALID_INTENTS = (
    "driver.",
    "runtime.",
    "package.",
)


def parse_task(payload: Dict[str, Any]) -> DispatchTask:
    intent = payload.get("intent", "")
    if not isinstance(intent, str) or not intent:
        raise ValueError("intent must be a non-empty string")
    if not intent.startswith(VALID_INTENTS):
        raise ValueError("intent must start with driver., runtime., or package.")
    return DispatchTask(
        intent=intent,
        host=payload.get("host", {}) or {},
        target=payload.get("target", {}) or {},
        context=payload.get("context", {}) or {},
    )
