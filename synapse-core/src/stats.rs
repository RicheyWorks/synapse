use chrono::{Duration, NaiveDate, Utc};
use serde::Serialize;

use crate::domain::MemoryItem;

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Stats {
    pub total_items: usize,
    pub due_now: usize,
    pub total_reviews: usize,
    /// Fraction of all reviews that were successful (score >= 3). 0.0 if no reviews yet.
    pub retention_rate: f32,
    pub current_streak_days: u32,
    pub best_streak_days: u32,
    pub average_ease: f32,
    pub leech_count: usize,
}

pub fn compute_stats(items: &[MemoryItem]) -> Stats {
    let total_items = items.len();
    let due_now = items.iter().filter(|i| i.is_due()).count();
    let leech_count = items.iter().filter(|i| i.is_leech()).count();

    let average_ease = if total_items == 0 {
        0.0
    } else {
        items.iter().map(|i| i.ease_factor).sum::<f32>() / total_items as f32
    };

    let all_reviews: Vec<_> = items.iter().flat_map(|i| i.review_log.iter()).collect();
    let total_reviews = all_reviews.len();
    let retention_rate = if total_reviews == 0 {
        0.0
    } else {
        all_reviews.iter().filter(|r| r.score >= 3).count() as f32 / total_reviews as f32
    };

    let review_dates = unique_sorted_review_dates(items);
    let current_streak_days = current_streak(&review_dates);
    let best_streak_days = best_streak(&review_dates);

    Stats {
        total_items,
        due_now,
        total_reviews,
        retention_rate,
        current_streak_days,
        best_streak_days,
        average_ease,
        leech_count,
    }
}

fn unique_sorted_review_dates(items: &[MemoryItem]) -> Vec<NaiveDate> {
    let mut dates: Vec<NaiveDate> = items
        .iter()
        .flat_map(|i| i.review_log.iter())
        .map(|entry| entry.reviewed_at.date_naive())
        .collect();
    dates.sort();
    dates.dedup();
    dates
}

fn best_streak(dates: &[NaiveDate]) -> u32 {
    if dates.is_empty() {
        return 0;
    }
    let mut best = 1u32;
    let mut current = 1u32;
    for pair in dates.windows(2) {
        if pair[1] - pair[0] == Duration::days(1) {
            current += 1;
        } else {
            current = 1;
        }
        best = best.max(current);
    }
    best
}

fn current_streak(dates: &[NaiveDate]) -> u32 {
    let Some(&last) = dates.last() else {
        return 0;
    };

    let today = Utc::now().date_naive();
    if today - last > Duration::days(1) {
        return 0;
    }

    let mut count = 1u32;
    let mut cursor = last;
    for &date in dates.iter().rev().skip(1) {
        if cursor - date == Duration::days(1) {
            count += 1;
            cursor = date;
        } else {
            break;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::{Scheduler, Sm2Scheduler};

    #[test]
    fn empty_collection_has_zeroed_stats() {
        let stats = compute_stats(&[]);
        assert_eq!(stats.total_items, 0);
        assert_eq!(stats.retention_rate, 0.0);
        assert_eq!(stats.current_streak_days, 0);
        assert_eq!(stats.best_streak_days, 0);
    }

    #[test]
    fn tracks_retention_and_leeches_across_items() {
        let scheduler = Sm2Scheduler;
        let mut good = MemoryItem::new("Rust", "What is a slice?", "...");
        scheduler.schedule(&mut good, 5);
        scheduler.schedule(&mut good, 5);

        let mut leech = MemoryItem::new("Rust", "What is variance?", "...");
        for _ in 0..8 {
            scheduler.schedule(&mut leech, 1);
        }

        let stats = compute_stats(&[good, leech]);
        assert_eq!(stats.total_items, 2);
        assert_eq!(stats.total_reviews, 10);
        assert_eq!(stats.leech_count, 1);
        assert!((stats.retention_rate - 0.2).abs() < 1e-6);
    }

    #[test]
    fn streaks_count_consecutive_review_days() {
        let mut item = MemoryItem::new("Rust", "prompt", "content");
        let today = Utc::now();
        item.review_log = vec![
            crate::domain::ReviewLogEntry {
                reviewed_at: today - Duration::days(2),
                score: 5,
                interval_before_days: 0,
                interval_after_days: 1,
                ease_factor_after: 2.5,
            },
            crate::domain::ReviewLogEntry {
                reviewed_at: today - Duration::days(1),
                score: 5,
                interval_before_days: 1,
                interval_after_days: 6,
                ease_factor_after: 2.5,
            },
            crate::domain::ReviewLogEntry {
                reviewed_at: today,
                score: 5,
                interval_before_days: 6,
                interval_after_days: 10,
                ease_factor_after: 2.5,
            },
        ];

        let stats = compute_stats(&[item]);
        assert_eq!(stats.current_streak_days, 3);
        assert_eq!(stats.best_streak_days, 3);
    }

    #[test]
    fn streak_breaks_after_a_missed_day() {
        let mut item = MemoryItem::new("Rust", "prompt", "content");
        let today = Utc::now();
        item.review_log = vec![
            crate::domain::ReviewLogEntry {
                reviewed_at: today - Duration::days(5),
                score: 5,
                interval_before_days: 0,
                interval_after_days: 1,
                ease_factor_after: 2.5,
            },
            crate::domain::ReviewLogEntry {
                reviewed_at: today,
                score: 5,
                interval_before_days: 1,
                interval_after_days: 6,
                ease_factor_after: 2.5,
            },
        ];

        let stats = compute_stats(&[item]);
        assert_eq!(stats.current_streak_days, 1);
        assert_eq!(stats.best_streak_days, 1);
    }
}
