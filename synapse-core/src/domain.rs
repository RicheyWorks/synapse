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

/// The actual recall material for a card. Tagged so the frontend can render
/// (and the add/edit form can collect) the right shape per type.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CardContent {
    /// Plain prompt -> answer.
    Basic { answer: String },
    /// `text` contains `{{c1::hidden phrase}}` style cloze deletions.
    Cloze { text: String },
    /// Syntax-highlighted code snippet as the answer.
    Code { language: String, code: String },
    /// An image (by path/URL) as the answer, with an optional caption.
    Image { path: String, caption: Option<String> },
}

impl CardContent {
    pub fn basic(answer: impl Into<String>) -> Self {
        CardContent::Basic {
            answer: answer.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryItem {
    pub id: String,
    pub training_track: String, // e.g., "Rust Programming", "Deep Learning"
    pub prompt: String,         // What triggers the recall
    pub card: CardContent,      // The actual knowledge/fact to recall
    pub repetitions: u32,       // How many times successfully reviewed
    pub ease_factor: f32,       // Difficulty modifier (starts at 2.5)
    pub interval_days: u32,     // Days until next review
    pub next_review: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    /// Older persisted items won't have this field; default to empty on load.
    #[serde(default)]
    pub review_log: Vec<ReviewLogEntry>,
    /// Consecutive failed reviews (score < 3), reset to 0 on the next success.
    #[serde(default)]
    pub lapses: u32,
    /// Lifetime failed-review count; never resets. Drives leech detection.
    #[serde(default)]
    pub total_lapses: u32,
    /// Ids of other `MemoryItem`s this one is conceptually linked to. Links
    /// are kept symmetric: linking A to B adds B to A's list and vice versa.
    #[serde(default)]
    pub related_ids: Vec<String>,
}

/// A memory that keeps failing review is a "leech": it's costing more review
/// time than it's worth and should be flagged for rewriting or suspension.
pub const LEECH_THRESHOLD: u32 = 8;

impl MemoryItem {
    pub fn new(training_track: &str, prompt: &str, card: CardContent) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            training_track: training_track.to_string(),
            prompt: prompt.to_string(),
            card,
            repetitions: 0,
            ease_factor: 2.5,
            interval_days: 0,
            next_review: now,
            created_at: now,
            review_log: Vec::new(),
            lapses: 0,
            total_lapses: 0,
            related_ids: Vec::new(),
        }
    }

    /// Checks if this specific memory is ready to be drilled/reviewed right now.
    pub fn is_due(&self) -> bool {
        Utc::now() >= self.next_review
    }

    /// True once this item has failed enough times to be flagged as a leech.
    pub fn is_leech(&self) -> bool {
        self.total_lapses >= LEECH_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_item_is_immediately_due() {
        let item = MemoryItem::new(
            "Rust",
            "What is ownership?",
            CardContent::basic("Exclusive access to a value"),
        );
        assert!(item.is_due());
        assert_eq!(item.repetitions, 0);
        assert_eq!(item.ease_factor, 2.5);
        assert!(item.review_log.is_empty());
        assert!(item.related_ids.is_empty());
        assert!(!item.is_leech());
    }

    #[test]
    fn item_becomes_leech_at_threshold() {
        let mut item = MemoryItem::new("Rust", "What is variance?", CardContent::basic("..."));
        item.total_lapses = LEECH_THRESHOLD - 1;
        assert!(!item.is_leech());
        item.total_lapses = LEECH_THRESHOLD;
        assert!(item.is_leech());
    }

    #[test]
    fn card_content_variants_round_trip_through_json() {
        let variants = vec![
            CardContent::basic("plain answer"),
            CardContent::Cloze {
                text: "The {{c1::mitochondria}} is the powerhouse of the cell".to_string(),
            },
            CardContent::Code {
                language: "rust".to_string(),
                code: "fn main() {}".to_string(),
            },
            CardContent::Image {
                path: "diagram.png".to_string(),
                caption: Some("Cell diagram".to_string()),
            },
        ];

        for card in variants {
            let json = serde_json::to_string(&card).unwrap();
            let restored: CardContent = serde_json::from_str(&json).unwrap();
            assert_eq!(card, restored);
        }
    }
}
