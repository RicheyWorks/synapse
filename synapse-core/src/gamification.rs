use serde::Serialize;

use crate::domain::MemoryItem;
use crate::stats::Stats;

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Achievement {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub unlocked: bool,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct GamificationSummary {
    pub xp: u32,
    pub level: u32,
    pub title: String,
    pub achievements: Vec<Achievement>,
}

/// XP rewards consistent effort over raw luck: every review counts, and a
/// correct one counts more. There's no persisted XP field — it's always
/// derived fresh from review history, so it can't drift out of sync.
fn compute_xp(stats: &Stats) -> u32 {
    let successful_reviews = (stats.retention_rate * stats.total_reviews as f32).round() as u32;
    stats.total_reviews as u32 * 10 + successful_reviews * 5
}

fn title_for_level(level: u32) -> &'static str {
    match level {
        0..=2 => "Landlubber",
        3..=5 => "Deckhand",
        6..=10 => "First Mate",
        11..=20 => "Captain",
        _ => "Blackbeard's Own",
    }
}

pub fn compute_gamification(items: &[MemoryItem], stats: &Stats) -> GamificationSummary {
    let xp = compute_xp(stats);
    let level = xp / 500;
    let title = title_for_level(level).to_string();

    let achievements = vec![
        Achievement {
            id: "first_blood",
            title: "First Blood",
            description: "Complete your first review",
            unlocked: stats.total_reviews >= 1,
        },
        Achievement {
            id: "iron_will",
            title: "Iron Will",
            description: "Hit a 7-day review streak",
            unlocked: stats.best_streak_days >= 7,
        },
        Achievement {
            id: "century_club",
            title: "Century Club",
            description: "Complete 100 total reviews",
            unlocked: stats.total_reviews >= 100,
        },
        Achievement {
            id: "sharpshooter",
            title: "Sharpshooter",
            description: "90%+ retention over at least 20 reviews",
            unlocked: stats.total_reviews >= 20 && stats.retention_rate >= 0.9,
        },
        Achievement {
            id: "leech_slayer",
            title: "Leech Slayer",
            description: "Zero leeches with at least 10 memories in the vault",
            unlocked: items.len() >= 10 && stats.leech_count == 0,
        },
        Achievement {
            id: "cartographer",
            title: "Cartographer",
            description: "Link at least 5 memories into your knowledge graph",
            unlocked: items.iter().filter(|i| !i.related_ids.is_empty()).count() >= 5,
        },
    ];

    GamificationSummary {
        xp,
        level,
        title,
        achievements,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::CardContent;

    fn stats_with(total_reviews: usize, retention_rate: f32, best_streak_days: u32, leech_count: usize) -> Stats {
        Stats {
            total_items: 0,
            due_now: 0,
            total_reviews,
            retention_rate,
            current_streak_days: 0,
            best_streak_days,
            average_ease: 2.5,
            leech_count,
        }
    }

    #[test]
    fn fresh_vault_has_zero_xp_and_lowest_title() {
        let stats = stats_with(0, 0.0, 0, 0);
        let summary = compute_gamification(&[], &stats);
        assert_eq!(summary.xp, 0);
        assert_eq!(summary.level, 0);
        assert_eq!(summary.title, "Landlubber");
        assert!(summary.achievements.iter().all(|a| !a.unlocked));
    }

    #[test]
    fn xp_and_achievements_grow_with_activity() {
        let stats = stats_with(100, 0.95, 10, 0);
        let items: Vec<MemoryItem> = (0..10)
            .map(|i| MemoryItem::new("Rust", &format!("q{i}"), CardContent::basic("...")))
            .collect();

        let summary = compute_gamification(&items, &stats);
        assert!(summary.xp > 0);
        assert!(summary.level >= 1);

        let unlocked: Vec<&str> = summary
            .achievements
            .iter()
            .filter(|a| a.unlocked)
            .map(|a| a.id)
            .collect();
        assert!(unlocked.contains(&"first_blood"));
        assert!(unlocked.contains(&"iron_will"));
        assert!(unlocked.contains(&"century_club"));
        assert!(unlocked.contains(&"sharpshooter"));
        assert!(unlocked.contains(&"leech_slayer"));
    }
}
