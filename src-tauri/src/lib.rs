#[cfg(feature = "desktop")]
mod desktop;
pub mod domain;
pub mod providers;
pub mod storage;

#[cfg(feature = "desktop")]
pub use desktop::run;

#[cfg(test)]
mod tests {
    use crate::domain::{
        ActivityCertainty, CandidateReason, SourceHealth, SourceId, candidate_reasons,
    };
    use crate::providers::default_root;
    use std::path::Path;

    #[test]
    fn unknown_activity_is_never_safe_to_trash() {
        assert!(!ActivityCertainty::Unknown.allows_cleanup());
        assert!(!ActivityCertainty::Running.allows_cleanup());
        assert!(ActivityCertainty::Inactive.allows_cleanup());
    }

    #[test]
    fn unknown_project_access_is_not_a_missing_project() {
        assert!(candidate_reasons(Some("/denied"), None, None, 100).is_empty());
        assert_eq!(
            candidate_reasons(None, None, None, 100),
            vec![CandidateReason::Unassociated]
        );
    }

    #[test]
    fn candidate_reasons_preserve_distinct_signals_and_ninety_day_boundary() {
        let now = 100 * 86_400;
        assert_eq!(
            candidate_reasons(Some("/missing"), Some(false), Some(now - 90 * 86_400), now),
            vec![
                CandidateReason::MissingProject,
                CandidateReason::Inactive90Days
            ]
        );
        assert!(
            candidate_reasons(Some("/present"), Some(true), Some(now - 89 * 86_400), now)
                .is_empty()
        );
    }

    #[test]
    fn documented_sources_have_default_roots() {
        let home = Path::new("/Users/tester");
        assert_eq!(
            default_root(SourceId::ClaudeCode, home),
            Some(home.join(".claude/projects"))
        );
        assert_eq!(
            default_root(SourceId::Codex, home),
            Some(home.join(".codex"))
        );
        assert_eq!(
            default_root(SourceId::DeepseekHarness, home),
            Some(home.join(".dsh/sessions"))
        );
        assert_eq!(
            default_root(SourceId::Pi, home),
            Some(home.join(".pi/agent/sessions"))
        );
    }

    #[test]
    fn enums_use_stable_wire_names() {
        assert_eq!(
            serde_json::to_string(&SourceId::DeepseekHarness).unwrap(),
            "\"deepseek_harness\""
        );
        assert_eq!(
            serde_json::to_string(&SourceHealth::NeedsValidation).unwrap(),
            "\"needs_validation\""
        );
    }
}
