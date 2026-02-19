use regex::Regex;
use std::sync::OnceLock;

struct PiiPatterns {
    email: Regex,
    phone: Regex,
    ssn: Regex,
}

static PII_PATTERNS: OnceLock<PiiPatterns> = OnceLock::new();

fn patterns() -> &'static PiiPatterns {
    PII_PATTERNS.get_or_init(|| PiiPatterns {
        email: Regex::new(r"(?i)\b[\w.+\-]+@[\w\-]+\.[\w.]+\b").unwrap(),
        phone: Regex::new(r"\b\d{3}[.\-]?\d{3}[.\-]?\d{4}\b").unwrap(),
        ssn: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
    })
}

/// Replace all detected PII (email addresses, phone numbers, SSNs) in `text`
/// with the literal string `[REDACTED]`.
///
/// Patterns are applied in the order: SSN → phone → email.  The SSN pattern is
/// applied first because it is the most specific dash-separated numeric pattern
/// and prevents the looser phone regex from consuming SSN digit groups.
pub fn mask_pii(text: &str) -> String {
    let p = patterns();

    let s = p.ssn.replace_all(text, "[REDACTED]");
    let s = p.phone.replace_all(&s, "[REDACTED]");
    let s = p.email.replace_all(&s, "[REDACTED]");

    s.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 1. Email is masked ─────────────────────────────────────────────────────

    #[test]
    fn masks_email() {
        assert_eq!(mask_pii("hello@test.com"), "[REDACTED]");
    }

    #[test]
    fn masks_email_in_sentence() {
        let result = mask_pii("contact support@example.org now");
        assert!(!result.contains("support@example.org"));
        assert!(result.contains("[REDACTED]"));
    }

    // ── 2. Phone number is masked ──────────────────────────────────────────────

    #[test]
    fn masks_phone_dashes() {
        assert_eq!(mask_pii("call 555-867-5309 today"), "call [REDACTED] today");
    }

    #[test]
    fn masks_phone_dots() {
        assert_eq!(
            mask_pii("reach me at 555.867.5309"),
            "reach me at [REDACTED]"
        );
    }

    #[test]
    fn masks_phone_plain() {
        assert_eq!(mask_pii("number is 5558675309"), "number is [REDACTED]");
    }

    // ── 3. SSN is masked ───────────────────────────────────────────────────────

    #[test]
    fn masks_ssn() {
        assert_eq!(mask_pii("SSN: 123-45-6789"), "SSN: [REDACTED]");
    }

    #[test]
    fn masks_ssn_in_sentence() {
        let result = mask_pii("Employee SSN is 078-05-1120.");
        assert!(!result.contains("078-05-1120"));
        assert!(result.contains("[REDACTED]"));
    }

    // ── 4. Text without PII is unchanged ──────────────────────────────────────

    #[test]
    fn no_pii_unchanged() {
        let clean = "The quick brown fox jumps over the lazy dog.";
        assert_eq!(mask_pii(clean), clean);
    }

    #[test]
    fn empty_string_unchanged() {
        assert_eq!(mask_pii(""), "");
    }

    // ── 5. Multiple PII types in same text ────────────────────────────────────

    #[test]
    fn multiple_pii_all_masked() {
        let text = "Email me at alice@example.com or call 800-555-1234. My SSN is 123-45-6789.";
        let result = mask_pii(text);
        assert!(!result.contains("alice@example.com"));
        assert!(!result.contains("800-555-1234"));
        assert!(!result.contains("123-45-6789"));
        assert_eq!(result.matches("[REDACTED]").count(), 3);
    }

    #[test]
    fn multiple_emails_all_masked() {
        let text = "From: alice@a.com To: bob@b.org";
        let result = mask_pii(text);
        assert_eq!(result.matches("[REDACTED]").count(), 2);
    }
}
