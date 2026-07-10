use std::collections::BTreeMap;

use chrono::{Duration, NaiveDate, Utc};
use serde::Serialize;

use crate::domain::MemoryItem;

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct TrackSummary {
    pub name: String,
    pub total: usize,
    pub due: usize,
}

/// Per-track item/due counts, sorted alphabetically by track name.
pub fn list_tracks(items: &[MemoryItem]) -> Vec<TrackSummary> {
    let mut counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for item in items {
        let entry = counts.entry(item.training_track.as_str()).or_default();
        entry.0 += 1;
        if item.is_due() {
            entry.1 += 1;
        }
    }
    counts
        .into_iter()
        .map(|(name, (total, due))| TrackSummary {
            name: name.to_string(),
            total,
            due,
        })
        .collect()
}

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

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct HeatmapDay {
    pub date: NaiveDate,
    pub review_count: usize,
}

/// Review counts per calendar day for the last `days` days (inclusive of
/// today), oldest first — suitable for a GitHub-style contribution heatmap.
/// Days with no reviews are included with a count of 0 so the frontend
/// doesn't have to fill gaps itself.
pub fn review_heatmap(items: &[MemoryItem], days: u32) -> Vec<HeatmapDay> {
    let mut counts: BTreeMap<NaiveDate, usize> = BTreeMap::new();
    for entry in items.iter().flat_map(|i| i.review_log.iter()) {
        *counts.entry(entry.reviewed_at.date_naive()).or_insert(0) += 1;
    }

    let today = Utc::now().date_naive();
    let start = today - Duration::days(days.saturating_sub(1) as i64);
    let mut out = Vec::with_capacity(days as usize);
    let mut cursor = start;
    while cursor <= today {
        out.push(HeatmapDay {
            date: cursor,
            review_count: counts.get(&cursor).copied().unwrap_or(0),
        });
        cursor += Duration::days(1);
    }
    out
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct RetentionPoint {
    pub date: NaiveDate,
    pub reviews: usize,
    pub retention_rate: f32,
}

/// Daily retention rate (fraction of that day's reviews with score >= 3),
/// oldest first. Days with zero reviews are omitted rather than shown as 0%,
/// since "no data" and "0% retention" are different things.
pub fn retention_over_time(items: &[MemoryItem]) -> Vec<RetentionPoint> {
    let mut by_day: BTreeMap<NaiveDate, (usize, usize)> = BTreeMap::new();
    for entry in items.iter().flat_map(|i| i.review_log.iter()) {
        let day = by_day.entry(entry.reviewed_at.date_naive()).or_insert((0, 0));
        day.0 += 1;
        if entry.score >= 3 {
            day.1 += 1;
        }
    }

    by_day
        .into_iter()
        .map(|(date, (total, successes))| RetentionPoint {
            date,
            reviews: total,
            retention_rate: successes as f32 / total as f32,
        })
        .collect()
}

/// A synthetic Ebbinghaus-style forgetting curve for a single item: estimated
/// probability of recall at each day offset from now, given its current
/// interval and ease as a proxy for memory stability. This is a projection
/// for the UI, not a measurement — it resets to ~100% at t=0 by construction.
pub fn forgetting_curve(item: &MemoryItem, days_ahead: u32) -> Vec<(u32, f32)> {
    let stability = (item.interval_days.max(1) as f32) * item.ease_factor;
    (0..=days_ahead)
        .map(|t| (t, (-(t as f32) / stability).exp()))
        .collect()
}

/// The `n` items with the most lifetime lapses, worst first — the ones
/// costing the most review time relative to what they teach.
pub fn hardest_items(items: &[MemoryItem], n: usize) -> Vec<MemoryItem> {
    let mut sorted: Vec<MemoryItem> = items.to_vec();
    sorted.sort_by(|a, b| b.total_lapses.cmp(&a.total_lapses));
    sorted.truncate(n);
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::CardContent;
    use crate::scheduler::{Scheduler, Sm2Scheduler};

    #[test]
    fn list_tracks_groups_and_counts_due_items() {
        let mut rust_due = MemoryItem::new("Rust", "What is a trait object?", CardContent::basic("..."));
        rust_due.next_review = Utc::now() - Duration::days(1);
        let mut rust_not_due = MemoryItem::new("Rust", "What is Copy?", CardContent::basic("..."));
        rust_not_due.next_review = Utc::now() + Duration::days(1);
        let mut bio = MemoryItem::new("Biology", "What is ATP?", CardContent::basic("..."));
        bio.next_review = Utc::now() - Duration::days(1);

        let tracks = list_tracks(&[rust_due, rust_not_due, bio]);

        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].name, "Biology");
        assert_eq!(tracks[0].total, 1);
        assert_eq!(tracks[0].due, 1);
        assert_eq!(tracks[1].name, "Rust");
        assert_eq!(tracks[1].total, 2);
        assert_eq!(tracks[1].due, 1);
    }

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
        let mut good = MemoryItem::new("Rust", "What is a slice?", CardContent::basic("..."));
        scheduler.schedule(&mut good, 5);
        scheduler.schedule(&mut good, 5);

        let mut leech = MemoryItem::new("Rust", "What is variance?", CardContent::basic("..."));
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
        let mut item = MemoryItem::new("Rust", "prompt", CardContent::basic("content"));
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
        let mut item = MemoryItem::new("Rust", "prompt", CardContent::basic("content"));
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

    #[test]
    fn heatmap_covers_every_day_in_range_including_zero_days() {
        let mut item = MemoryItem::new("Rust", "prompt", CardContent::basic("content"));
        item.review_log = vec![crate::domain::ReviewLogEntry {
            reviewed_at: Utc::now(),
            score: 5,
            interval_before_days: 0,
            interval_after_days: 1,
            ease_factor_after: 2.5,
        }];

        let heatmap = review_heatmap(&[item], 7);
        assert_eq!(heatmap.len(), 7);
        assert_eq!(heatmap.last().unwrap().review_count, 1);
        assert_eq!(heatmap.first().unwrap().review_count, 0);
    }

    #[test]
    fn retention_over_time_groups_by_day_and_omits_empty_days() {
        let mut item = MemoryItem::new("Rust", "prompt", CardContent::basic("content"));
        let today = Utc::now();
        item.review_log = vec![
            crate::domain::ReviewLogEntry {
                reviewed_at: today,
                score: 5,
                interval_before_days: 0,
                interval_after_days: 1,
                ease_factor_after: 2.5,
            },
            crate::domain::ReviewLogEntry {
                reviewed_at: today,
                score: 1,
                interval_before_days: 1,
                interval_after_days: 1,
                ease_factor_after: 2.5,
            },
        ];

        let curve = retention_over_time(&[item]);
        assert_eq!(curve.len(), 1);
        assert_eq!(curve[0].reviews, 2);
        assert!((curve[0].retention_rate - 0.5).abs() < 1e-6);
    }

    #[test]
    fn forgetting_curve_starts_at_full_recall_and_decays() {
        let mut item = MemoryItem::new("Rust", "prompt", CardContent::basic("content"));
        item.interval_days = 10;
        item.ease_factor = 2.5;

        let curve = forgetting_curve(&item, 30);
        assert_eq!(curve[0], (0, 1.0));
        assert!(curve.last().unwrap().1 < curve[0].1);
        assert!(curve.windows(2).all(|w| w[1].1 <= w[0].1));
    }

    #[test]
    fn hardest_items_sorts_by_lifetime_lapses_descending() {
        let mut low = MemoryItem::new("Rust", "easy", CardContent::basic("..."));
        low.total_lapses = 1;
        let mut high = MemoryItem::new("Rust", "hard", CardContent::basic("..."));
        high.total_lapses = 9;

        let worst = hardest_items(&[low, high], 1);
        assert_eq!(worst.len(), 1);
        assert_eq!(worst[0].prompt, "hard");
    }
}
