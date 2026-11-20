//! Capture filters and pipeline (specs/03 §3.2, specs/04 §P-05).
// Integration-test helpers are test code (REL-004 applies to production code only).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use clipvault_application::capture::{
    Candidate, CaptureContext, CaptureFilter, CapturePipeline, ContentSelector, Deduplicator,
    EmptyFilter, ExcludedAppFilter, FilterOutcome, OwnWriteFilter, OwnWriteRegistry, PausedFilter,
    PrivacyFlagsFilter, SizeLimitFilter,
};
use clipvault_application::ports::ClipboardSnapshot;
use clipvault_domain::app_name::AppName;
use clipvault_domain::capture::{CaptureDecision, CapturedContent, IgnoreReason};
use clipvault_domain::hash::ContentHash;
use clipvault_domain::settings::CaptureSettings;
use clipvault_test_support::{InMemoryHistoryRepository, snapshot};

fn ctx(snapshot: ClipboardSnapshot) -> CaptureContext {
    CaptureContext::new(snapshot, CaptureSettings::default())
}

fn ctx_with(snapshot: ClipboardSnapshot, settings: CaptureSettings) -> CaptureContext {
    CaptureContext::new(snapshot, settings)
}

fn stopped(filter: &dyn CaptureFilter, ctx: CaptureContext) -> Option<CaptureDecision> {
    match filter.apply(ctx).unwrap() {
        FilterOutcome::Stop(decision) => Some(decision),
        FilterOutcome::Continue(_) => None,
    }
}

fn continued(filter: &dyn CaptureFilter, ctx: CaptureContext) -> CaptureContext {
    match filter.apply(ctx).unwrap() {
        FilterOutcome::Continue(ctx) => ctx,
        FilterOutcome::Stop(decision) => panic!("expected Continue, got Stop({decision:?})"),
    }
}

const IGNORE_PAUSED: Option<CaptureDecision> = Some(CaptureDecision::Ignore(IgnoreReason::Paused));

#[test]
fn cap_008_paused_filter_ignores_changes_while_paused() {
    let paused = CaptureSettings {
        paused: true,
        ..CaptureSettings::default()
    };

    assert_eq!(
        stopped(&PausedFilter, ctx_with(snapshot::text("x").build(), paused)),
        IGNORE_PAUSED
    );
    assert_eq!(
        stopped(&PausedFilter, ctx(snapshot::text("x").build())),
        None
    );
}

#[test]
fn cap_010_own_write_filter_ignores_registered_sequence_once() {
    let registry = Arc::new(OwnWriteRegistry::default());
    registry.record(snapshot::sequence(42));
    let filter = OwnWriteFilter::new(Arc::clone(&registry));

    assert_eq!(
        stopped(&filter, ctx(snapshot::text("x").sequence(42).build())),
        Some(CaptureDecision::Ignore(IgnoreReason::OwnWrite))
    );
    assert_eq!(
        stopped(&filter, ctx(snapshot::text("x").sequence(42).build())),
        None,
        "consumed"
    );
    assert_eq!(
        stopped(&filter, ctx(snapshot::text("x").sequence(43).build())),
        None
    );
}

#[test]
fn cap_007_privacy_flags_filter_ignores_any_opt_out_format() {
    let ignored = Some(CaptureDecision::Ignore(IgnoreReason::PrivacyFlag));

    for flagged in [
        snapshot::text("secret").exclude_from_monitor().build(),
        snapshot::text("secret").viewer_ignore().build(),
        snapshot::text("secret").history_disallowed().build(),
    ] {
        assert_eq!(stopped(&PrivacyFlagsFilter, ctx(flagged)), ignored);
    }
    assert_eq!(
        stopped(&PrivacyFlagsFilter, ctx(snapshot::text("ok").build())),
        None
    );
}

#[test]
fn cap_009_excluded_app_filter_ignores_copies_from_excluded_apps() {
    let settings = CaptureSettings {
        excluded_apps: vec![AppName::parse("keepass.exe").unwrap()],
        ..CaptureSettings::default()
    };
    let from = |owner: Option<&str>| {
        let builder = snapshot::text("x");
        ctx_with(
            owner.map_or(builder.clone(), |o| builder.owner(o)).build(),
            settings.clone(),
        )
    };

    assert_eq!(
        stopped(&ExcludedAppFilter, from(Some("KeePass.exe"))),
        Some(CaptureDecision::Ignore(IgnoreReason::ExcludedApp))
    );
    assert_eq!(stopped(&ExcludedAppFilter, from(Some("notepad.exe"))), None);
    assert_eq!(
        stopped(&ExcludedAppFilter, from(None)),
        None,
        "unknown owner is captured"
    );
}

#[test]
fn cap_003_selector_prefers_text_over_image() {
    let ctx = continued(
        &ContentSelector,
        ctx(snapshot::text("célula").image(2, 2).build()),
    );

    assert_eq!(ctx.candidate, Some(Candidate::Text("célula".into())));
}

#[test]
fn cap_003_selector_falls_back_to_image_when_text_is_blank() {
    let ctx = continued(
        &ContentSelector,
        ctx(snapshot::text("  \n").image(2, 2).build()),
    );

    assert!(matches!(ctx.candidate, Some(Candidate::Image(ref px)) if px.width() == 2));
}

#[test]
fn cap_005_empty_filter_ignores_changes_without_usable_content() {
    for empty in [snapshot::empty().build(), snapshot::text(" \t ").build()] {
        let ctx = continued(&ContentSelector, ctx(empty));
        assert_eq!(
            stopped(&EmptyFilter, ctx),
            Some(CaptureDecision::Ignore(IgnoreReason::Empty))
        );
    }
}

#[test]
fn cap_006_size_limit_filter_ignores_text_above_the_limit() {
    let settings = CaptureSettings {
        max_text_bytes: 4,
        ..CaptureSettings::default()
    };
    let with_text = |text: &str| {
        let mut ctx = ctx_with(snapshot::empty().build(), settings.clone());
        ctx.candidate = Some(Candidate::Text(text.into()));
        ctx
    };

    assert_eq!(
        stopped(&SizeLimitFilter, with_text("abcd")),
        None,
        "at the limit"
    );
    assert_eq!(
        stopped(&SizeLimitFilter, with_text("abcde")),
        Some(CaptureDecision::Ignore(IgnoreReason::TooLarge))
    );
}

#[test]
fn cap_006_size_limit_filter_ignores_images_above_the_limit() {
    let settings = CaptureSettings {
        max_image_bytes: 16,
        ..CaptureSettings::default()
    };
    let with_image = |w, h| {
        continued(
            &ContentSelector,
            ctx_with(snapshot::empty().image(w, h).build(), settings.clone()),
        )
    };

    assert_eq!(
        stopped(&SizeLimitFilter, with_image(2, 2)),
        None,
        "16 bytes: at the limit"
    );
    assert_eq!(
        stopped(&SizeLimitFilter, with_image(3, 2)),
        Some(CaptureDecision::Ignore(IgnoreReason::TooLarge))
    );
}

#[test]
fn cap_004_deduplicator_stores_new_content_with_hash_and_source() {
    let history = Arc::new(InMemoryHistoryRepository::default());
    let filter = Deduplicator::new(history);
    let ctx = continued(
        &ContentSelector,
        ctx(snapshot::text("novo").owner("Code.exe").build()),
    );

    let Some(CaptureDecision::Store(clip)) = stopped(&filter, ctx) else {
        panic!("expected Store");
    };

    assert_eq!(clip.hash, ContentHash::of_text("novo"));
    assert!(matches!(clip.content, CapturedContent::Text(ref t) if t.as_str() == "novo"));
    assert_eq!(clip.source_app, Some(AppName::parse("code.exe").unwrap()));
}

#[test]
fn cap_004_deduplicator_promotes_existing_content() {
    let history = Arc::new(InMemoryHistoryRepository::default());
    let existing = history.seed_text("repetido");
    let filter = Deduplicator::new(history);
    let ctx = continued(&ContentSelector, ctx(snapshot::text("repetido").build()));

    assert_eq!(
        stopped(&filter, ctx),
        Some(CaptureDecision::Promote(existing))
    );
}

#[test]
fn deduplicator_without_candidate_ignores_as_empty() {
    let filter = Deduplicator::new(Arc::new(InMemoryHistoryRepository::default()));

    assert_eq!(
        stopped(&filter, ctx(snapshot::empty().build())),
        Some(CaptureDecision::Ignore(IgnoreReason::Empty))
    );
}

#[test]
fn standard_pipeline_applies_filters_in_the_specified_order() {
    let pipeline = CapturePipeline::standard(
        Arc::new(OwnWriteRegistry::default()),
        Arc::new(InMemoryHistoryRepository::default()),
    );

    assert_eq!(
        pipeline.filter_names(),
        [
            "paused",
            "own-write",
            "privacy-flags",
            "excluded-app",
            "content-selector",
            "empty",
            "size-limit",
            "deduplicator",
        ]
    );
}

#[test]
fn cap_007_cap_008_earlier_filters_short_circuit_later_ones() {
    let pipeline = CapturePipeline::standard(
        Arc::new(OwnWriteRegistry::default()),
        Arc::new(InMemoryHistoryRepository::default()),
    );
    let paused = CaptureSettings {
        paused: true,
        ..CaptureSettings::default()
    };

    let decision = pipeline
        .evaluate(ctx_with(
            snapshot::text("secret").exclude_from_monitor().build(),
            paused,
        ))
        .unwrap();

    assert_eq!(decision, CaptureDecision::Ignore(IgnoreReason::Paused));
}

#[test]
fn a_pipeline_that_never_stops_ignores_the_change() {
    let pipeline = CapturePipeline::new(vec![Box::new(PausedFilter)]);

    assert_eq!(
        pipeline.evaluate(ctx(snapshot::text("x").build())).unwrap(),
        CaptureDecision::Ignore(IgnoreReason::Empty)
    );
}
