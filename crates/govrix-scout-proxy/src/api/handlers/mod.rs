//! REST API handler modules.
//!
//! Each handler module corresponds to a resource:
//! - `health`   — liveness / readiness checks
//! - `events`   — event log queries
//! - `agents`   — agent registry CRUD
//! - `costs`    — cost aggregation
//! - `budgets`  — per-agent budget configuration
//! - `reports`  — report generation (stub)
//! - `config`   — runtime config read
//! - `projects` — project management CRUD

pub mod agents;
pub mod budgets;
pub mod config;
pub mod costs;
pub mod events;
pub mod health;
pub mod projects;
pub mod reports;
