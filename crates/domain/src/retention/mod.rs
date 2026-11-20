//! History retention rules (RET-001..003). Pure functions; the SQL implementation in the
//! infrastructure layer is tested against [`RetentionPolicy::select_expired`] as its oracle.
//!
//! Pattern: Strategy — specs/04 §P-06 (age and count rules composed into one policy).

use std::collections::BTreeSet;
use std::time::Duration;

use crate::ids::ClipId;
use crate::time::Timestamp;

const DAY: Duration = Duration::from_hours(24);

/// Errors converting stored settings into retention options.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RetentionError {
    /// A number of days that is not one of the offered options.
    #[error("unsupported retention period: {0} days")]
    UnsupportedPeriod(u32),
    /// A maximum item count that is not one of the offered options.
    #[error("unsupported maximum item count: {0}")]
    UnsupportedMaxItems(u32),
}

/// How long unused items are kept (RET-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RetentionPeriod {
    /// 1 day.
    OneDay,
    /// 7 days.
    SevenDays,
    /// 30 days (default).
    #[default]
    ThirtyDays,
    /// 90 days.
    NinetyDays,
    /// 365 days.
    OneYear,
    /// Never expire by age.
    Forever,
}

impl RetentionPeriod {
    /// All options, in display order.
    pub const ALL: [Self; 6] = [
        Self::OneDay,
        Self::SevenDays,
        Self::ThirtyDays,
        Self::NinetyDays,
        Self::OneYear,
        Self::Forever,
    ];

    /// Length in days, `None` for [`Self::Forever`] (`retention.max_age_days`, specs/05 §3).
    pub const fn days(self) -> Option<u32> {
        match self {
            Self::OneDay => Some(1),
            Self::SevenDays => Some(7),
            Self::ThirtyDays => Some(30),
            Self::NinetyDays => Some(90),
            Self::OneYear => Some(365),
            Self::Forever => None,
        }
    }

    /// Maximum age of an unused item, `None` for no limit.
    pub fn max_age(self) -> Option<Duration> {
        self.days().map(|d| DAY * d)
    }

    /// Parses a stored number of days.
    ///
    /// # Errors
    /// [`RetentionError::UnsupportedPeriod`] for values that are not an offered option.
    pub fn from_days(days: Option<u32>) -> Result<Self, RetentionError> {
        Self::ALL
            .into_iter()
            .find(|p| p.days() == days)
            .ok_or(RetentionError::UnsupportedPeriod(days.unwrap_or_default()))
    }
}

/// Maximum number of unpinned items kept (RET-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaxItems {
    /// 100 items.
    Hundred,
    /// 500 items.
    FiveHundred,
    /// 1,000 items (default).
    #[default]
    Thousand,
    /// 5,000 items.
    FiveThousand,
    /// No limit.
    Unlimited,
}

impl MaxItems {
    /// All options, in display order.
    pub const ALL: [Self; 5] = [
        Self::Hundred,
        Self::FiveHundred,
        Self::Thousand,
        Self::FiveThousand,
        Self::Unlimited,
    ];

    /// The limit, `None` for [`Self::Unlimited`] (`retention.max_items`, specs/05 §3).
    pub const fn limit(self) -> Option<u32> {
        match self {
            Self::Hundred => Some(100),
            Self::FiveHundred => Some(500),
            Self::Thousand => Some(1_000),
            Self::FiveThousand => Some(5_000),
            Self::Unlimited => None,
        }
    }

    /// Parses a stored limit.
    ///
    /// # Errors
    /// [`RetentionError::UnsupportedMaxItems`] for values that are not an offered option.
    pub fn from_count(count: Option<u32>) -> Result<Self, RetentionError> {
        Self::ALL.into_iter().find(|m| m.limit() == count).ok_or(
            RetentionError::UnsupportedMaxItems(count.unwrap_or_default()),
        )
    }
}

/// The minimal view of a history item needed to apply retention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionCandidate {
    /// Item id.
    pub id: ClipId,
    /// Last time the item was captured or pasted.
    pub last_used_at: Timestamp,
    /// Pinned items are exempt (RET-003).
    pub pinned: bool,
}

/// Combined age and count rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RetentionPolicy {
    /// Age rule.
    pub period: RetentionPeriod,
    /// Count rule.
    pub max_items: MaxItems,
}

impl RetentionPolicy {
    /// Ids of the items to remove, in input order (specs/05 §5):
    /// unpinned items older than the period, plus the least recently used unpinned items
    /// beyond the limit among the remaining ones.
    pub fn select_expired(&self, items: &[RetentionCandidate], now: Timestamp) -> Vec<ClipId> {
        let max_age = self.period.max_age();
        let is_expired = |c: &RetentionCandidate| {
            max_age.is_some_and(|age| now.elapsed_since(c.last_used_at) > age)
        };

        let mut removed: BTreeSet<ClipId> = items
            .iter()
            .filter(|c| !c.pinned && is_expired(c))
            .map(|c| c.id)
            .collect();

        if let Some(limit) = self.max_items.limit() {
            let mut alive: Vec<&RetentionCandidate> = items
                .iter()
                .filter(|c| !c.pinned && !removed.contains(&c.id))
                .collect();
            alive.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at).then(b.id.cmp(&a.id)));
            removed.extend(alive.iter().skip(limit as usize).map(|c| c.id));
        }

        items
            .iter()
            .filter(|c| removed.contains(&c.id))
            .map(|c| c.id)
            .collect()
    }
}

#[cfg(test)]
mod tests;
