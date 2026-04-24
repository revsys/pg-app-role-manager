use std::fmt;

/// Outcome of an individual action
#[derive(Debug, Clone)]
pub enum ActionOutcome {
    Created,
    Skipped,
    Updated,
    Moved,
}

impl fmt::Display for ActionOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionOutcome::Created => write!(f, "Created"),
            ActionOutcome::Skipped => write!(f, "Skipped"),
            ActionOutcome::Updated => write!(f, "Updated"),
            ActionOutcome::Moved => write!(f, "Moved"),
        }
    }
}

/// Collects actions during command execution
#[derive(Debug)]
pub struct ActionReport {
    command_name: String,
    actions: Vec<(String, ActionOutcome)>,
}

impl ActionReport {
    pub fn new(command_name: impl Into<String>) -> Self {
        Self {
            command_name: command_name.into(),
            actions: Vec::new(),
        }
    }

    /// Record an action with immediate console output
    pub fn record(&mut self, description: impl Into<String>, outcome: ActionOutcome) {
        let desc = description.into();
        println!("{}: {}", outcome, desc);
        self.actions.push((desc, outcome));
    }

    /// Format the summary report as a string (testable without stdout)
    pub fn format_summary(&self) -> String {
        let created = self.actions.iter().filter(|(_, o)| matches!(o, ActionOutcome::Created)).count();
        let skipped = self.actions.iter().filter(|(_, o)| matches!(o, ActionOutcome::Skipped)).count();
        let updated = self.actions.iter().filter(|(_, o)| matches!(o, ActionOutcome::Updated)).count();
        let moved   = self.actions.iter().filter(|(_, o)| matches!(o, ActionOutcome::Moved)).count();

        let mut lines: Vec<String> = vec![
            String::new(),
            format!("=== {} Summary ===", self.command_name),
            format!("Total actions: {}", self.actions.len()),
        ];

        if created > 0 { lines.push(format!("  Created: {}", created)); }
        if skipped > 0 { lines.push(format!("  Skipped: {}", skipped)); }
        if updated > 0 { lines.push(format!("  Updated: {}", updated)); }
        if moved   > 0 { lines.push(format!("  Moved: {}",   moved)); }

        lines.push("==================".to_string());
        lines.join("\n")
    }

    /// Print the summary report
    pub fn print_summary(&self) {
        println!("{}", self.format_summary());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ActionOutcome display ────────────────────────────────────────────────

    #[test]
    fn outcome_display_created() {
        assert_eq!(ActionOutcome::Created.to_string(), "Created");
    }

    #[test]
    fn outcome_display_skipped() {
        assert_eq!(ActionOutcome::Skipped.to_string(), "Skipped");
    }

    #[test]
    fn outcome_display_updated() {
        assert_eq!(ActionOutcome::Updated.to_string(), "Updated");
    }

    #[test]
    fn outcome_display_moved() {
        assert_eq!(ActionOutcome::Moved.to_string(), "Moved");
    }

    // ── ActionReport summary ─────────────────────────────────────────────────

    #[test]
    fn empty_summary_shows_zero_total() {
        let report = ActionReport::new("Test");
        let summary = report.format_summary();
        assert!(summary.contains("Total actions: 0"));
    }

    #[test]
    fn empty_summary_omits_per_type_lines() {
        let report = ActionReport::new("Test");
        let summary = report.format_summary();
        assert!(!summary.contains("Created:"));
        assert!(!summary.contains("Skipped:"));
        assert!(!summary.contains("Updated:"));
        assert!(!summary.contains("Moved:"));
    }

    #[test]
    fn counts_one_of_each_outcome() {
        let mut report = ActionReport::new("Test");
        report.record("a", ActionOutcome::Created);
        report.record("b", ActionOutcome::Skipped);
        report.record("c", ActionOutcome::Updated);
        report.record("d", ActionOutcome::Moved);
        let summary = report.format_summary();
        assert!(summary.contains("Total actions: 4"));
        assert!(summary.contains("Created: 1"));
        assert!(summary.contains("Skipped: 1"));
        assert!(summary.contains("Updated: 1"));
        assert!(summary.contains("Moved: 1"));
    }

    #[test]
    fn omits_zero_count_types() {
        let mut report = ActionReport::new("Test");
        report.record("x", ActionOutcome::Moved);
        report.record("y", ActionOutcome::Moved);
        let summary = report.format_summary();
        assert!(summary.contains("Moved: 2"));
        assert!(!summary.contains("Created:"));
        assert!(!summary.contains("Skipped:"));
        assert!(!summary.contains("Updated:"));
    }

    #[test]
    fn summary_contains_command_name() {
        let report = ActionReport::new("Rehome");
        let summary = report.format_summary();
        assert!(summary.contains("=== Rehome Summary ==="));
    }
}
