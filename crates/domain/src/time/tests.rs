use std::time::Duration;

use super::*;

#[test]
fn elapsed_since_is_the_positive_difference() {
    let earlier = Timestamp::from_millis(1_000);
    let later = Timestamp::from_millis(3_500);

    assert_eq!(later.elapsed_since(earlier), Duration::from_millis(2_500));
}

#[test]
fn elapsed_since_saturates_at_zero_when_clock_goes_backwards() {
    let earlier = Timestamp::from_millis(1_000);
    let later = Timestamp::from_millis(3_500);

    assert_eq!(earlier.elapsed_since(later), Duration::ZERO);
}

#[test]
fn timestamps_are_ordered_and_round_trip_millis() {
    assert!(Timestamp::from_millis(1) < Timestamp::from_millis(2));
    assert_eq!(Timestamp::from_millis(42).as_millis(), 42);
}

#[test]
fn adding_a_duration_moves_forward() {
    let t = Timestamp::from_millis(1_000);

    assert_eq!(
        t + Duration::from_millis(250),
        Timestamp::from_millis(1_250)
    );
}
