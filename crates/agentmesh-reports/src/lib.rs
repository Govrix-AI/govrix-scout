//! AgentMesh Reports — HTML, JSON, and PDF report generation.
//!
//! Uses minijinja for HTML/text templating.
//! PDF generation (via printable HTML or a PDF crate) comes in Phase 4.
//!
//! Report types:
//! - `summary`     — overall agent activity summary
//! - `compliance`  — audit trail with lineage hash verification
//! - `cost`        — cost breakdown by agent, model, and day
//! - `pii`         — PII detection findings (type + location, no values)

pub mod cost;
pub mod usage;

pub use cost::CostBreakdown;
pub use usage::UsageSummary;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Report output format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    Json,
    Html,
    /// PDF stub — requires Phase 4 implementation
    Pdf,
}

/// A generated report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Unique report ID (UUIDv7).
    pub id: uuid::Uuid,
    /// Report type name.
    pub report_type: String,
    /// The time range covered by this report.
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    /// Format this report was generated in.
    pub format: ReportFormat,
    /// The generated content (HTML string or JSON string).
    pub content: String,
    /// When this report was generated.
    pub generated_at: DateTime<Utc>,
}

/// Report generation context passed to templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportContext {
    pub title: String,
    pub from: String,
    pub to: String,
    pub generated_at: String,
    pub data: serde_json::Value,
    /// Upsell footer (required per build spec).
    pub upsell_url: String,
}

impl Default for ReportContext {
    fn default() -> Self {
        Self {
            title: String::new(),
            from: String::new(),
            to: String::new(),
            generated_at: Utc::now().to_rfc3339(),
            data: serde_json::Value::Null,
            upsell_url: "https://agentmesh.io".to_string(),
        }
    }
}

/// Generate a report from the given context using minijinja.
///
/// The template string is rendered with `context` as the template variables.
/// Returns the rendered string.
///
/// # Example
/// ```
/// use agentmesh_reports::{ReportContext, render_template};
/// let ctx = ReportContext {
///     title: "Summary Report".to_string(),
///     ..Default::default()
/// };
/// let output = render_template("Hello {{ title }}!", &ctx).unwrap();
/// assert!(output.contains("Summary Report"));
/// ```
pub fn render_template(
    template_str: &str,
    context: &ReportContext,
) -> Result<String, minijinja::Error> {
    let mut env = minijinja::Environment::new();
    env.add_template("report", template_str)?;
    let tmpl = env.get_template("report")?;
    let ctx_value = minijinja::Value::from_serialize(context);
    tmpl.render(ctx_value)
}

/// Built-in summary report HTML template.
pub const SUMMARY_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>{{ title }}</title>
  <style>
    body { font-family: system-ui, sans-serif; max-width: 900px; margin: 2rem auto; }
    footer { margin-top: 3rem; font-size: 0.8rem; color: #666; }
  </style>
</head>
<body>
  <h1>{{ title }}</h1>
  <p>Period: {{ from }} — {{ to }}</p>
  <p>Generated: {{ generated_at }}</p>
  <hr>
  <pre>{{ data }}</pre>
  <footer>
    Powered by AgentMesh OSS.
    <a href="{{ upsell_url }}">Upgrade to AgentMesh Enterprise</a>
    for compliance policy enforcement, unlimited retention, and A2A identity.
  </footer>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_summary_template() {
        let ctx = ReportContext {
            title: "Test Report".to_string(),
            from: "2026-01-01".to_string(),
            to: "2026-01-07".to_string(),
            ..Default::default()
        };
        let rendered = render_template(SUMMARY_TEMPLATE, &ctx).expect("render failed");
        assert!(rendered.contains("Test Report"));
        assert!(rendered.contains("agentmesh.io"));
    }
}
