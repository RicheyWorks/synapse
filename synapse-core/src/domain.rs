use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single completed review of a `MemoryItem`, kept for history/analytics.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReviewLogEntry {
    pub reviewed_at: DateTime<Utc>,
    pub score: u8,
    pub interval_before_days: u32,
    pub interval_after_days: u32,
    pub ease_factor_after: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryItem {
    pub id: String,
    pub training_track: String, // e.g., "Rust Programming", "Deep Learning"
    pub prompt: String,         // What triggers the recall
    pub content: String,        // The actual knowledge/fact
    pub repetitions: u32,       // How many times successfully reviewed
    pub ease_factor: f32,       // Difficulty modifier (starts at 2.5)
    pub interval_days: u32,     // Days until next review
    pub next_review: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    /// Older persisted items won't have this field; default to empty on load.
    #[serde(default)]
    pub review_log: Vec<ReviewLogEntry>,
}

impl MemoryItem {
    pub fn new(training_track: &str, prompt: &str, content: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            training_track: training_track.to_string(),
            prompt: prompt.to_string(),
            content: content.to_string(),
            repetitions: 0,
            ease_factor: 2.5,
            interval_days: 0,
            next_review: now,
            created_at: now,
            review_log: Vec::new(),
        }
    }

    /// Checks if this specific memory is ready to be drilled/reviewed right now.
    pub fn is_due(&self) -> bool {
        Utc::now() >= self.next_review
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_item_is_immediately_due() {
        let item = MemoryItem::new("Rust", "What is ownership?", "Exclusive access to a value");
        assert!(item.is_due());
        assert_eq!(item.repetitions, 0);
        assert_eq!(item.ease_factor, 2.5);
        assert!(item.review_log.is_empty());
    }
}
