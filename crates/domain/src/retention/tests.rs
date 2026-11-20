use std::time::Duration;

use proptest::prelude::*;

use super::*;

const DAY: Duration = Duration::from_hours(24);

fn item(id: i64, last_used_ms: i64, pinned: bool) -> RetentionCandidate {
    RetentionCandidate {
        id: ClipId::new(id),
        last_used_at: Timestamp::from_millis(last_used_ms),
        pinned,
    }
}

fn ids(list: &[i64]) -> Vec<ClipId> {
    list.iter().copied().map(ClipId::new).collect()
}

fn days_ms(days: u64) -> i64 {
    i64::try_from((DAY * u32::try_from(days).unwrap()).as_millis()).unwrap()
}

#[test]
fn default_policy_is_thirty_days_and_one_thousand_items() {
    let policy = RetentionPolicy::default();

    assert_eq!(policy.period, RetentionPeriod::ThirtyDays);
    assert_eq!(policy.max_items, MaxItems::Thousand);
}

#[test]
fn ret_001_period_options_map_to_max_age() {
    assert_eq!(RetentionPeriod::OneDay.max_age(), Some(DAY));
    assert_eq!(RetentionPeriod::SevenDays.max_age(), Some(DAY * 7));
    assert_eq!(RetentionPeriod::ThirtyDays.max_age(), Some(DAY * 30));
    assert_eq!(RetentionPeriod::NinetyDays.max_age(), Some(DAY * 90));
    assert_eq!(RetentionPeriod::OneYear.max_age(), Some(DAY * 365));
    assert_eq!(RetentionPeriod::Forever.max_age(), None);
}

#[test]
fn ret_001_period_round_trips_through_days_and_rejects_other_values() {
    for period in RetentionPeriod::ALL {
        assert_eq!(RetentionPeriod::from_days(period.days()), Ok(period));
    }
    assert_eq!(
        RetentionPeriod::from_days(Some(2)),
        Err(RetentionError::UnsupportedPeriod(2))
    );
}

#[test]
fn ret_002_max_items_round_trips_through_count_and_rejects_other_values() {
    for max in MaxItems::ALL {
        assert_eq!(MaxItems::from_count(max.limit()), Ok(max));
    }
    assert_eq!(
        MaxItems::from_count(Some(7)),
        Err(RetentionError::UnsupportedMaxItems(7))
    );
}

#[test]
fn ret_001_items_older_than_the_period_are_selected() {
    let policy = RetentionPolicy {
        period: RetentionPeriod::OneDay,
        max_items: MaxItems::Unlimited,
    };
    let now = Timestamp::from_millis(days_ms(10));
    let items = [
        item(1, days_ms(9), false),     // exactly one day old: kept
        item(2, days_ms(9) - 1, false), // one day + 1 ms: expired
        item(3, days_ms(1), false),     // long expired
        item(4, days_ms(10), false),    // fresh
    ];

    assert_eq!(policy.select_expired(&items, now), ids(&[2, 3]));
}

#[test]
fn ret_001_forever_never_expires_by_age() {
    let policy = RetentionPolicy {
        period: RetentionPeriod::Forever,
        max_items: MaxItems::Unlimited,
    };
    let items = [item(1, 0, false)];

    assert_eq!(
        policy.select_expired(&items, Timestamp::from_millis(days_ms(5000))),
        vec![]
    );
}

#[test]
fn ret_002_least_recently_used_items_beyond_the_limit_are_selected() {
    let policy = RetentionPolicy {
        period: RetentionPeriod::Forever,
        max_items: MaxItems::Hundred,
    };
    let items: Vec<_> = (1..=103).map(|i| item(i, i * 10, false)).collect();

    assert_eq!(
        policy.select_expired(&items, Timestamp::from_millis(10_000)),
        ids(&[1, 2, 3])
    );
}

#[test]
fn ret_003_pinned_items_never_expire_nor_count_towards_the_limit() {
    let policy = RetentionPolicy {
        period: RetentionPeriod::OneDay,
        max_items: MaxItems::Hundred,
    };
    let now = Timestamp::from_millis(days_ms(100));
    let mut items: Vec<_> = (1..=100).map(|i| item(i, days_ms(100), false)).collect();
    items.push(item(500, 0, true)); // ancient but pinned
    items.push(item(501, days_ms(100), true));

    assert_eq!(policy.select_expired(&items, now), vec![]);
}

#[test]
fn ret_001_ret_002_items_selected_by_both_rules_are_reported_once() {
    let policy = RetentionPolicy {
        period: RetentionPeriod::OneDay,
        max_items: MaxItems::Hundred,
    };
    let now = Timestamp::from_millis(days_ms(10));
    let mut items: Vec<_> = (1..=100).map(|i| item(i, days_ms(10), false)).collect();
    items.push(item(200, 0, false)); // expired by age and also beyond the limit

    assert_eq!(policy.select_expired(&items, now), ids(&[200]));
}

fn policy_strategy() -> impl Strategy<Value = RetentionPolicy> {
    (
        prop::sample::select(RetentionPeriod::ALL.to_vec()),
        prop::sample::select(MaxItems::ALL.to_vec()),
    )
        .prop_map(|(period, max_items)| RetentionPolicy { period, max_items })
}

fn items_strategy() -> impl Strategy<Value = Vec<RetentionCandidate>> {
    prop::collection::vec((0..days_ms(400), any::<bool>()), 0..300).prop_map(|rows| {
        rows.into_iter()
            .zip(1..)
            .map(|((last_used, pinned), id)| item(id, last_used, pinned))
            .collect()
    })
}

proptest! {
    #[test]
    fn ret_003_selection_never_contains_pinned_items(policy in policy_strategy(), items in items_strategy()) {
        let selected = policy.select_expired(&items, Timestamp::from_millis(days_ms(400)));
        for candidate in items.iter().filter(|i| i.pinned) {
            prop_assert!(!selected.contains(&candidate.id));
        }
    }

    #[test]
    fn ret_002_remaining_unpinned_items_respect_the_limit(policy in policy_strategy(), items in items_strategy()) {
        let selected = policy.select_expired(&items, Timestamp::from_millis(days_ms(400)));
        let remaining = items.iter().filter(|i| !i.pinned && !selected.contains(&i.id)).count();
        if let Some(limit) = policy.max_items.limit() {
            prop_assert!(remaining <= limit as usize);
        }
    }

    #[test]
    fn ret_001_every_remaining_item_is_within_the_period(policy in policy_strategy(), items in items_strategy()) {
        let now = Timestamp::from_millis(days_ms(400));
        let selected = policy.select_expired(&items, now);
        if let Some(max_age) = policy.period.max_age() {
            for kept in items.iter().filter(|i| !i.pinned && !selected.contains(&i.id)) {
                prop_assert!(now.elapsed_since(kept.last_used_at) <= max_age);
            }
        }
    }

    #[test]
    fn selection_is_minimal(policy in policy_strategy(), items in items_strategy()) {
        // Nothing is removed unless required: the selection is exactly the expired items plus
        // the excess over the limit among the rest.
        let now = Timestamp::from_millis(days_ms(400));
        let selected = policy.select_expired(&items, now);
        let expired_by_age = items
            .iter()
            .filter(|i| !i.pinned && policy.period.max_age().is_some_and(|age| now.elapsed_since(i.last_used_at) > age))
            .count();
        let alive = items.iter().filter(|i| !i.pinned).count() - expired_by_age;
        let excess = policy.max_items.limit().map_or(0, |limit| alive.saturating_sub(limit as usize));
        prop_assert_eq!(selected.len(), expired_by_age + excess);
    }
}
