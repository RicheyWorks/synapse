//! FSRS-6 (Free Spaced Repetition Scheduler), a second `Scheduler` implementation
//! alongside `Sm2Scheduler`.
//!
//! Formulas and default parameters are ported from the official reference
//! implementation, `open-spaced-repetition/py-fsrs` (v6.3.1, `fsrs/scheduler.py`),
//! MIT licensed. This is a faithful port of that crate's **Review-state**
//! formulas only: py-fsrs also models a separate Learning/Relearning state
//! machine with sub-day, minutes-granularity "learning steps" for brand-new
//! cards. This app is day-granularity throughout (matching `Sm2Scheduler`,
//! which also always schedules at least one full day out), so that step-queue
//! mechanic is intentionally not ported — every review here uses the formulas
//! py-fsrs applies to a card already in its Review state, including the
//! same-day ("short-term") stability formula for the edge case of two reviews
//! within 24 hours of each other.
//!
//! This app's UI uses a 0-5 quality scale (Anki/SM-2 convention); FSRS uses a
//! 4-point Again/Hard/Good/Easy scale. `fsrs_rating` maps one onto the other
//! so both schedulers share the same review UI and `Scheduler::schedule` signature.

use chrono::Utc;

use crate::domain::{MemoryItem, ReviewLogEntry};
use crate::scheduler::Scheduler;

/// Default weights trained by the FSRS maintainers on a large aggregate review
/// dataset ("should be excellent right out of the box" per the project docs).
/// Index meanings, per py-fsrs:
/// - `w[0..4]`: initial stability by rating (Again/Hard/Good/Easy)
/// - `w[4..6]`: initial difficulty
/// - `w[6..8]`: difficulty update (delta + mean reversion)
/// - `w[8..11]`: recall stability growth
/// - `w[11..15]`: forget (lapse) stability
/// - `w[15..17]`: hard penalty / easy bonus
/// - `w[17..20]`: same-day ("short-term") stability
/// - `w[20]`: personalized forgetting-curve decay
const DEFAULT_PARAMETERS: [f64; 21] = [
    0.212, 1.2931, 2.3065, 8.2956, 6.4133, 0.8334, 3.0194, 0.001, 1.8722, 0.1666, 0.796, 1.4835, 0.0614,
    0.2629, 1.6483, 0.6014, 1.8729, 0.5425, 0.0912, 0.0658, 0.1542,
];

const STABILITY_MIN: f64 = 0.001;
const MIN_DIFFICULTY: f64 = 1.0;
const MAX_DIFFICULTY: f64 = 10.0;
const MAXIMUM_INTERVAL_DAYS: i64 = 36_500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FsrsRating {
    Again = 1,
    Hard = 2,
    Good = 3,
    Easy = 4,
}

/// Maps this app's 0-5 review score onto FSRS's 4-point rating scale.
fn fsrs_rating(score: u8) -> FsrsRating {
    match score.min(5) {
        0..=2 => FsrsRating::Again,
        3 => FsrsRating::Hard,
        4 => FsrsRating::Good,
        _ => FsrsRating::Easy,
    }
}

pub struct FsrsScheduler {
    parameters: [f64; 21],
    /// Target probability of recall at the scheduled review date (0.7-0.99).
    /// Higher means shorter, more frequent reviews.
    desired_retention: f64,
}

impl Default for FsrsScheduler {
    fn default() -> Self {
        Self {
            parameters: DEFAULT_PARAMETERS,
            desired_retention: 0.9,
        }
    }
}

impl FsrsScheduler {
    pub fn new(desired_retention: f64) -> Self {
        Self {
            parameters: DEFAULT_PARAMETERS,
            desired_retention: desired_retention.clamp(0.7, 0.99),
        }
    }

    fn decay(&self) -> f64 {
        -self.parameters[20]
    }

    fn factor(&self) -> f64 {
        0.9f64.powf(1.0 / self.decay()) - 1.0
    }

    fn clamp_stability(s: f64) -> f64 {
        s.max(STABILITY_MIN)
    }

    fn clamp_difficulty(d: f64) -> f64 {
        d.clamp(MIN_DIFFICULTY, MAX_DIFFICULTY)
    }

    fn initial_stability(&self, rating: FsrsRating) -> f64 {
        Self::clamp_stability(self.parameters[rating as usize - 1])
    }

    fn initial_difficulty(&self, rating: FsrsRating, clamp: bool) -> f64 {
        let d = self.parameters[4] - (self.parameters[5] * (rating as i32 as f64 - 1.0)).exp() + 1.0;
        if clamp {
            Self::clamp_difficulty(d)
        } else {
            d
        }
    }

    fn next_difficulty(&self, difficulty: f64, rating: FsrsRating) -> f64 {
        let delta_difficulty = -(self.parameters[6] * (rating as i32 as f64 - 3.0));
        let damped = difficulty + (10.0 - difficulty) * delta_difficulty / 9.0;
        let easy_anchor = self.initial_difficulty(FsrsRating::Easy, false);
        let reverted = self.parameters[7] * easy_anchor + (1.0 - self.parameters[7]) * damped;
        Self::clamp_difficulty(reverted)
    }

    /// Predicted probability of recall after `elapsed_days` since the last review.
    fn retrievability(&self, elapsed_days: f64, stability: f64) -> f64 {
        (1.0 + self.factor() * elapsed_days / stability).powf(self.decay())
    }

    fn next_interval_days(&self, stability: f64) -> u32 {
        let days = (stability / self.factor()) * (self.desired_retention.powf(1.0 / self.decay()) - 1.0);
        days.round().clamp(1.0, MAXIMUM_INTERVAL_DAYS as f64) as u32
    }

    /// Stability update for a same-day re-review (elapsed < 1 day).
    fn short_term_stability(&self, stability: f64, rating: FsrsRating) -> f64 {
        let r = rating as i32 as f64;
        let mut increase = (self.parameters[17] * (r - 3.0 + self.parameters[18])).exp() * stability.powf(-self.parameters[19]);
        if matches!(rating, FsrsRating::Good | FsrsRating::Easy) {
            increase = increase.max(1.0);
        }
        Self::clamp_stability(stability * increase)
    }

    fn next_forget_stability(&self, difficulty: f64, stability: f64, retrievability: f64) -> f64 {
        let long_term = self.parameters[11]
            * difficulty.powf(-self.parameters[12])
            * (((stability + 1.0).powf(self.parameters[13])) - 1.0)
            * ((1.0 - retrievability) * self.parameters[14]).exp();
        let short_term = stability / (self.parameters[17] * self.parameters[18]).exp();
        long_term.min(short_term)
    }

    fn next_recall_stability(&self, difficulty: f64, stability: f64, retrievability: f64, rating: FsrsRating) -> f64 {
        let hard_penalty = if rating == FsrsRating::Hard { self.parameters[15] } else { 1.0 };
        let easy_bonus = if rating == FsrsRating::Easy { self.parameters[16] } else { 1.0 };
        stability
            * (1.0
                + self.parameters[8].exp()
                    * (11.0 - difficulty)
                    * stability.powf(-self.parameters[9])
                    * (((1.0 - retrievability) * self.parameters[10]).exp() - 1.0)
                    * hard_penalty
                    * easy_bonus)
    }

    fn next_stability(&self, difficulty: f64, stability: f64, retrievability: f64, rating: FsrsRating) -> f64 {
        let s = match rating {
            FsrsRating::Again => self.next_forget_stability(difficulty, stability, retrievability),
            _ => self.next_recall_stability(difficulty, stability, retrievability, rating),
        };
        Self::clamp_stability(s)
    }
}

impl Scheduler for FsrsScheduler {
    fn schedule(&self, item: &mut MemoryItem, score: u8) {
        let score = score.min(5);
        let rating = fsrs_rating(score);
        let now = Utc::now();
        let interval_before = item.interval_days;
        let last_review = item.review_log.last().map(|entry| entry.reviewed_at);

        let (difficulty, stability) = match (item.difficulty, item.stability, last_review) {
            (Some(d), Some(s), Some(last)) => {
                let elapsed_days = (now - last).num_days().max(0);
                let next_d = self.next_difficulty(d, rating);
                let next_s = if elapsed_days < 1 {
                    self.short_term_stability(s, rating)
                } else {
                    let r = self.retrievability(elapsed_days as f64, s);
                    self.next_stability(d, s, r, rating)
                };
                (next_d, next_s)
            }
            _ => (self.initial_difficulty(rating, true), self.initial_stability(rating)),
        };

        item.difficulty = Some(difficulty);
        item.stability = Some(stability);
        item.interval_days = self.next_interval_days(stability);
        item.next_review = now + chrono::Duration::days(item.interval_days as i64);

        if rating == FsrsRating::Again {
            item.repetitions = 0;
            item.lapses += 1;
            item.total_lapses += 1;
        } else {
            item.repetitions += 1;
            item.lapses = 0;
        }

        item.review_log.push(ReviewLogEntry {
            reviewed_at: now,
            score,
            interval_before_days: interval_before,
            interval_after_days: item.interval_days,
            ease_factor_after: item.ease_factor,
            difficulty_after: Some(difficulty),
            stability_after: Some(stability),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::CardContent;

    fn item() -> MemoryItem {
        MemoryItem::new("Rust", "What is a trait?", CardContent::basic("A shared interface"))
    }

    #[test]
    fn first_review_uses_initial_stability_and_difficulty() {
        let scheduler = FsrsScheduler::default();
        let mut m = item();

        scheduler.schedule(&mut m, 4); // maps to FsrsRating::Good

        assert_eq!(m.stability, Some(DEFAULT_PARAMETERS[2])); // w[2] = initial stability for Good
        assert!(m.difficulty.unwrap() >= MIN_DIFFICULTY && m.difficulty.unwrap() <= MAX_DIFFICULTY);
        assert_eq!(m.repetitions, 1);
        assert_eq!(m.lapses, 0);
        assert_eq!(m.review_log.len(), 1);
        assert!(m.review_log[0].stability_after.is_some());
    }

    #[test]
    fn at_default_desired_retention_interval_approximately_equals_stability() {
        // FSRS's FACTOR is derived so that R(t=S) == desired_retention. When
        // desired_retention is exactly the default 0.9, next_interval(S) == S
        // by construction (a useful invariant to test against, rather than a
        // hand-computed magic number).
        let scheduler = FsrsScheduler::default();
        let stability = 12.3_f64;
        let interval = scheduler.next_interval_days(stability);
        assert!((interval as f64 - stability).abs() < 1.0);
    }

    #[test]
    fn again_rating_resets_repetitions_and_increments_lapses() {
        let scheduler = FsrsScheduler::default();
        let mut m = item();
        scheduler.schedule(&mut m, 4); // Good
        scheduler.schedule(&mut m, 0); // Again

        assert_eq!(m.repetitions, 0);
        assert_eq!(m.lapses, 1);
        assert_eq!(m.total_lapses, 1);
    }

    #[test]
    fn higher_desired_retention_yields_shorter_intervals() {
        let lenient = FsrsScheduler::new(0.99);
        let strict_stability = 10.0;
        let short_interval = lenient.next_interval_days(strict_stability);

        let relaxed = FsrsScheduler::new(0.8);
        let long_interval = relaxed.next_interval_days(strict_stability);

        assert!(short_interval < long_interval);
    }

    #[test]
    fn retrievability_is_1_at_zero_elapsed_and_decays_over_time() {
        let scheduler = FsrsScheduler::default();
        let r0 = scheduler.retrievability(0.0, 10.0);
        let r_later = scheduler.retrievability(30.0, 10.0);
        assert!((r0 - 1.0).abs() < 1e-9);
        assert!(r_later < r0);
    }

    #[test]
    fn repeated_good_reviews_grow_stability() {
        let scheduler = FsrsScheduler::default();
        let mut m = item();
        scheduler.schedule(&mut m, 4);
        let s1 = m.stability.unwrap();
        // Simulate as if the next review happens after the scheduled interval,
        // so this exercises the long-term (non-same-day) stability branch.
        if let Some(entry) = m.review_log.last_mut() {
            entry.reviewed_at = Utc::now() - chrono::Duration::days(m.interval_days as i64 + 1);
        }
        scheduler.schedule(&mut m, 4);
        let s2 = m.stability.unwrap();
        assert!(s2 > s1);
    }

    #[test]
    fn desired_retention_is_clamped_to_a_sane_range() {
        let too_high = FsrsScheduler::new(1.5);
        let too_low = FsrsScheduler::new(0.1);
        assert!(too_high.desired_retention <= 0.99);
        assert!(too_low.desired_retention >= 0.7);
    }
}
