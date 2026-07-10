use chrono::{Duration, Utc};

use crate::domain::{MemoryItem, ReviewLogEntry};

/// Strategy for scheduling the next review of a `MemoryItem`.
///
/// Kept as a trait rather than baked into `MemoryItem` so a new algorithm
/// (e.g. FSRS) can be dropped in later without touching domain or storage code.
pub trait Scheduler {
    /// Applies a review with the given quality `score` (0-5, Anki/SM-2 convention:
    /// 5 = perfect recall, 0 = total blackout) and updates the item in place.
    fn schedule(&self, item: &mut MemoryItem, score: u8);
}

/// Classic SM-2 spaced-repetition algorithm.
pub struct Sm2Scheduler;

impl Scheduler for Sm2Scheduler {
    fn schedule(&self, item: &mut MemoryItem, score: u8) {
        let score = score.min(5);
        let interval_before = item.interval_days;

        if score >= 3 {
            item.interval_days = match item.repetitions {
                0 => 1,
                1 => 6,
                _ => ((item.interval_days as f32) * item.ease_factor).round() as u32,
            };
            item.repetitions += 1;
            item.lapses = 0;
        } else {
            item.repetitions = 0;
            item.interval_days = 1;
            item.lapses += 1;
            item.total_lapses += 1;
        }

        let q = score as f32;
        item.ease_factor += 0.1 - (5.0 - q) * (0.08 + (5.0 - q) * 0.02);
        if item.ease_factor < 1.3 {
            item.ease_factor = 1.3;
        }

        let now = Utc::now();
        item.next_review = now + Duration::days(item.interval_days as i64);

        item.review_log.push(ReviewLogEntry {
            reviewed_at: now,
            score,
            interval_before_days: interval_before,
            interval_after_days: item.interval_days,
            ease_factor_after: item.ease_factor,
            difficulty_after: None,
            stability_after: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::CardContent;

    #[test]
    fn successful_reviews_grow_interval_and_log_history() {
        let mut item = MemoryItem::new("Rust", "What is a trait?", CardContent::basic("A shared interface"));
        let scheduler = Sm2Scheduler;

        scheduler.schedule(&mut item, 5);
        assert_eq!(item.repetitions, 1);
        assert_eq!(item.interval_days, 1);

        scheduler.schedule(&mut item, 5);
        assert_eq!(item.repetitions, 2);
        assert_eq!(item.interval_days, 6);

        scheduler.schedule(&mut item, 5);
        assert_eq!(item.repetitions, 3);
        assert!(item.interval_days > 6);

        assert_eq!(item.review_log.len(), 3);
        assert_eq!(item.review_log[1].interval_before_days, 1);
        assert_eq!(item.review_log[1].interval_after_days, 6);
    }

    #[test]
    fn failed_review_resets_repetitions_without_dropping_ease_below_floor() {
        let mut item = MemoryItem::new("Rust", "What is Send?", CardContent::basic("Safe to move across threads"));
        let scheduler = Sm2Scheduler;
        item.ease_factor = 1.3;

        scheduler.schedule(&mut item, 0);

        assert_eq!(item.repetitions, 0);
        assert_eq!(item.interval_days, 1);
        assert!(item.ease_factor >= 1.3);
        assert_eq!(item.lapses, 1);
        assert_eq!(item.total_lapses, 1);
    }

    #[test]
    fn a_success_resets_consecutive_lapses_but_not_lifetime_total() {
        let mut item = MemoryItem::new("Rust", "What is a lifetime?", CardContent::basic("..."));
        let scheduler = Sm2Scheduler;

        scheduler.schedule(&mut item, 1);
        scheduler.schedule(&mut item, 1);
        assert_eq!(item.lapses, 2);
        assert_eq!(item.total_lapses, 2);

        scheduler.schedule(&mut item, 5);
        assert_eq!(item.lapses, 0);
        assert_eq!(item.total_lapses, 2);
    }
}
