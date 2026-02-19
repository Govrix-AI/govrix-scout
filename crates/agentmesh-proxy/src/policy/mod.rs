//! Policy engine — compliance enforcement for intercepted agent traffic.
//!
//! This module provides the full policy evaluation pipeline:
//! - `types` — core enums and structs (`PolicyAction`, `PolicyDecision`, etc.)
//! - `pii`   — regex-based PII detection (email, phone, SSN, credit card, IP)
//! - `budget`— in-memory token/cost budget tracking per agent
//! - `loader`— YAML policy configuration loading
//! - `engine`— the `PolicyEngine` struct that ties everything together
//!
//! # Compliance-first invariant
//!
//! Every `PolicyDecision` carries a `compliance_tag` in `"{status}:{policy_name}"`
//! format per the compliance-first skill. The interceptor calls
//! `PolicyEngine::compute_compliance_tag(&decisions)` to derive the single tag
//! stored in `AgentEvent::compliance_tag`.
//!
//! # Fail-open design
//!
//! If policy evaluation fails (panic, mutex poison, file error), the engine
//! returns `"audit:error"` and traffic continues. Policy failures MUST NOT
//! block agent traffic in any code path.
//!
//! # Usage in the interceptor
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use agentmesh_proxy::policy::engine::PolicyEngine;
//!
//! // At startup — create once, share via Arc
//! let engine = Arc::new(PolicyEngine::noop());
//!
//! // In the proxy handler (fire-and-forget task)
//! let engine_clone = Arc::clone(&engine);
//! let event_clone = event.clone();
//! tokio::spawn(async move {
//!     let decisions = engine_clone.evaluate(&event_clone);
//!     let tag = PolicyEngine::compute_compliance_tag(&decisions);
//!     // Store tag in event record
//! });
//! ```

pub mod budget;
pub mod engine;
pub mod loader;
pub mod pii;
pub mod types;

// Re-export the most commonly used types at the module level for ergonomics.
// These are suppressed until the policy engine is wired into the proxy interceptor.
#[allow(unused_imports)]
pub use engine::PolicyEngine;
#[allow(unused_imports)]
pub use types::{AlertSeverity, PolicyAction, PolicyCondition, PolicyDecision, PolicyRule};
