use std::fmt;

/// Outcome of an individual action
#[derive(Debug, Clone)]
pub enum ActionOutcome {
    Created,
    Skipped,
    Updated,
    Removed,
    NotFound,
}

impl fmt::Display for ActionOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionOutcome::Created => write!(f, "Created"),
            ActionOutcome::Skipped => write!(f, "Skipped"),
            ActionOutcome::Updated => write!(f, "Updated"),
            ActionOutcome::Removed => write!(f, "Removed"),
            ActionOutcome::NotFound => write!(f, "Not Found"),
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

    /// Print the summary report
    pub fn print_summary(&self) {
        let created = self.actions.iter().filter(|(_, o)| matches!(o, ActionOutcome::Created)).count();
        let skipped = self.actions.iter().filter(|(_, o)| matches!(o, ActionOutcome::Skipped)).count();
        let updated = self.actions.iter().filter(|(_, o)| matches!(o, ActionOutcome::Updated)).count();
        let removed = self.actions.iter().filter(|(_, o)| matches!(o, ActionOutcome::Removed)).count();
        let not_found = self.actions.iter().filter(|(_, o)| matches!(o, ActionOutcome::NotFound)).count();

        println!();
        println!("=== {} Summary ===", self.command_name);
        println!("Total actions: {}", self.actions.len());

        if created > 0 {
            println!("  Created: {}", created);
        }
        if skipped > 0 {
            println!("  Skipped: {}", skipped);
        }
        if updated > 0 {
            println!("  Updated: {}", updated);
        }
        if removed > 0 {
            println!("  Removed: {}", removed);
        }
        if not_found > 0 {
            println!("  Not Found: {}", not_found);
        }

        println!("==================");
    }
}
