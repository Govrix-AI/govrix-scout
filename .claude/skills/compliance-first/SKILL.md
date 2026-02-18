---
name: compliance-first
description: Core compliance invariant for AgentMesh. Use when implementing ANY code that intercepts, logs, or processes agent actions. Every intercepted action MUST generate session ID, timestamp, data lineage hash, and policy-compliance tag.
compatibility: compliance, governance, audit, logging, proxy, interceptor
---

# AgentMesh Compliance-First Invariant

## The Rule (Non-Negotiable)

**Every agent action intercepted by AgentMesh MUST produce these four fields:**

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | UUID | Groups related actions in a single agent session/conversation |
| `timestamp` | DateTime (UTC, ms precision) | Exact time the action was intercepted |
| `lineage_hash` | SHA-256 hex string (64 chars) | Merkle-chain hash linking this event to its predecessor |
| `compliance_tag` | String | Policy evaluation result for this action |

**No exceptions.** If code intercepts an agent action and does not produce all four fields, it is a bug.

## Session ID

A session groups related agent actions into a logical unit (a conversation, a task, a workflow run).

```python
# Session assignment rules:
# 1. Same agent + same conversation_id header → same session
# 2. Same agent + requests within 5min window with no conversation_id → same session
# 3. New agent or >5min gap → new session

import uuid
from datetime import datetime, timedelta

class SessionTracker:
    def __init__(self, timeout_minutes: int = 5):
        self._sessions: dict[str, tuple[uuid.UUID, datetime]] = {}
        self._timeout = timedelta(minutes=timeout_minutes)

    def get_session_id(self, agent_id: str, conversation_id: str | None = None) -> uuid.UUID:
        key = f"{agent_id}:{conversation_id}" if conversation_id else agent_id

        if key in self._sessions:
            session_id, last_seen = self._sessions[key]
            if datetime.utcnow() - last_seen < self._timeout:
                self._sessions[key] = (session_id, datetime.utcnow())
                return session_id

        # New session
        session_id = uuid.uuid4()
        self._sessions[key] = (session_id, datetime.utcnow())
        return session_id
```

```rust
// Rust equivalent for the SaaS proxy
fn get_session_id(
    sessions: &DashMap<String, (Uuid, Instant)>,
    agent_id: &str,
    conversation_id: Option<&str>,
    timeout: Duration,
) -> Uuid {
    let key = match conversation_id {
        Some(cid) => format!("{}:{}", agent_id, cid),
        None => agent_id.to_string(),
    };

    if let Some(mut entry) = sessions.get_mut(&key) {
        let (session_id, last_seen) = entry.value();
        if last_seen.elapsed() < timeout {
            *entry.value_mut() = (*session_id, Instant::now());
            return *session_id;
        }
    }

    let session_id = Uuid::new_v4();
    sessions.insert(key, (session_id, Instant::now()));
    session_id
}
```

## Data Lineage Hash (Merkle Chain)

Each event's `lineage_hash` chains to the previous event for the same session, creating a tamper-evident audit trail.

```python
import hashlib
import json

def compute_lineage_hash(
    session_id: str,
    timestamp: str,
    event_data: dict,
    previous_hash: str | None = None,
) -> str:
    """
    Compute SHA-256 lineage hash for an event.

    The hash chains: previous_hash + session_id + timestamp + canonical(event_data)
    If previous_hash is None (first event in session), uses "GENESIS" as seed.

    This creates a Merkle chain per session — if any event is tampered with,
    all subsequent hashes break, making tampering detectable.
    """
    seed = previous_hash or "GENESIS"
    canonical = json.dumps(event_data, sort_keys=True, separators=(',', ':'))
    payload = f"{seed}|{session_id}|{timestamp}|{canonical}"
    return hashlib.sha256(payload.encode('utf-8')).hexdigest()
```

```rust
// Rust equivalent
use sha2::{Sha256, Digest};

fn compute_lineage_hash(
    session_id: &Uuid,
    timestamp: &str,
    event_data: &serde_json::Value,
    previous_hash: Option<&str>,
) -> String {
    let seed = previous_hash.unwrap_or("GENESIS");
    let canonical = serde_json::to_string(event_data).unwrap(); // deterministic
    let payload = format!("{}|{}|{}|{}", seed, session_id, timestamp, canonical);

    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

**Rules:**
- First event in a session uses `"GENESIS"` as the seed
- `event_data` MUST be canonicalized (sorted keys, no whitespace) before hashing
- The hash chain is per-session, not global
- Store the hash in the `lineage_hash` column (see agentmesh-schemas skill)
- To verify chain integrity: recompute hashes sequentially and compare

## Policy-Compliance Tag

Every event gets a compliance tag from the policy engine. The tag is the result of evaluating all applicable policies against the event.

```python
# Compliance tag format: "{status}:{policy_name}" or "{status}:none"
# Status values:
#   "pass"    — all policies passed
#   "warn"    — soft policy violation (logged, not blocked)
#   "block"   — hard policy violation (blocked in SaaS, flagged in OSS)
#   "audit"   — no policies assigned, logged for audit trail only

VALID_COMPLIANCE_STATUSES = {"pass", "warn", "block", "audit"}

def compute_compliance_tag(
    event: dict,
    applicable_policies: list[Policy],
) -> str:
    """Evaluate all policies and return the highest-severity tag."""
    if not applicable_policies:
        return "audit:none"

    worst_status = "pass"
    worst_policy = "all"
    severity_order = {"pass": 0, "warn": 1, "block": 2}

    for policy in applicable_policies:
        result = policy.evaluate(event)
        if severity_order.get(result.status, 0) > severity_order.get(worst_status, 0):
            worst_status = result.status
            worst_policy = policy.name

    return f"{worst_status}:{worst_policy}"
```

**Tag examples:**
- `pass:all` — all policies passed
- `warn:cost_budget` — cost budget policy triggered a warning
- `block:pii_redaction` — PII detected, blocked by redaction policy (SaaS only)
- `audit:none` — no policies, logged for audit

**In Scout OSS:** The compliance tag is always `"audit:none"` or `"pass:all"` (Scout has threshold alerts but no enforcement policies). The field MUST still be populated for forward compatibility with SaaS migration.

## The Interceptor Contract

Every interceptor (proxy, scanner, MCP bridge) must call this function:

```python
from dataclasses import dataclass
from datetime import datetime, timezone
from uuid import UUID

@dataclass
class ComplianceEnvelope:
    """The compliance envelope wraps every intercepted event."""
    session_id: UUID
    timestamp: str  # ISO 8601 UTC
    lineage_hash: str  # SHA-256 hex
    compliance_tag: str  # "{status}:{policy}"

def create_compliance_envelope(
    session_tracker: SessionTracker,
    lineage_store: LineageStore,
    policy_engine: PolicyEngine,
    agent_id: str,
    event_data: dict,
    conversation_id: str | None = None,
) -> ComplianceEnvelope:
    """
    MUST be called for every intercepted agent action.
    Returns the four compliance fields that are stored with the event.
    """
    # 1. Session ID
    session_id = session_tracker.get_session_id(agent_id, conversation_id)

    # 2. Timestamp (UTC, millisecond precision)
    timestamp = datetime.now(timezone.utc).isoformat(timespec='milliseconds')

    # 3. Lineage hash (chains to previous event in session)
    previous_hash = lineage_store.get_latest_hash(session_id)
    lineage_hash = compute_lineage_hash(
        str(session_id), timestamp, event_data, previous_hash
    )
    lineage_store.store_hash(session_id, lineage_hash)

    # 4. Compliance tag
    policies = policy_engine.get_policies_for_agent(agent_id)
    compliance_tag = compute_compliance_tag(event_data, policies)

    return ComplianceEnvelope(
        session_id=session_id,
        timestamp=timestamp,
        lineage_hash=lineage_hash,
        compliance_tag=compliance_tag,
    )
```

## Verification

To verify an audit trail:

```python
def verify_session_integrity(events: list[dict]) -> bool:
    """
    Verify the Merkle chain for a session's events.
    Events must be ordered by timestamp ascending.
    Returns True if chain is intact, False if tampered.
    """
    previous_hash = None
    for event in events:
        expected = compute_lineage_hash(
            event['session_id'],
            event['timestamp'],
            event['event_data'],
            previous_hash,
        )
        if expected != event['lineage_hash']:
            return False
        previous_hash = event['lineage_hash']
    return True
```

## Checklist for Any Interceptor Code

- [ ] `session_id` assigned via `SessionTracker` (not random per-request)
- [ ] `timestamp` is UTC with millisecond precision
- [ ] `lineage_hash` computed with `compute_lineage_hash()` chaining to previous
- [ ] `compliance_tag` computed by policy engine (or `"audit:none"` for Scout OSS)
- [ ] All four fields stored in the event record (see agentmesh-schemas)
- [ ] No PII values in the compliance envelope — only types and locations
- [ ] Fail-open: if compliance computation fails, use `"audit:error"` tag and continue
- [ ] Lineage hash uses canonical JSON (sorted keys, no whitespace)
