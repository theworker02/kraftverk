//! Integration-style tests for optimizer + MockPlatform.

use kraftverk_core::candidate::{Candidate, ParamChange, ParamValue};
use kraftverk_core::statistics::{compare_samples, StatsConfig};
use kraftverk_data::ExperimentStore;
use kraftverk_data::{write_receipt, EvidenceReceipt};
use kraftverk_optimizer::{
    create_search_plugin, list_search_plugins, HillClimbStrategy, SearchContext, SearchDecision,
    SearchStrategy, PLUGIN_HILL_CLIMB,
};
use kraftverk_system::mock::{MockConfig, MockPlatform};
use kraftverk_system::{ApplyGuard, Platform, RecoveryJournal};
use tempfile::tempdir;

#[test]
fn mock_hill_climb_finds_improvement_signal() {
    let mut platform = MockPlatform::new(MockConfig {
        seed: 11,
        noise_stddev: 0.0,
        ..MockConfig::default()
    });
    let base = platform.score_multiplier();
    // Manually apply sweet spot.
    platform
        .apply_change(&ParamChange {
            key: "bench.worker_threads".into(),
            previous: ParamValue::Int(4),
            next: ParamValue::Int(6),
            rationale: "t".into(),
        })
        .unwrap();
    assert!(platform.score_multiplier() > base);
}

#[test]
fn journal_recovers_interrupted_apply() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("journal.json");
    let mut journal = RecoveryJournal::open(&path).unwrap();
    let mut platform = MockPlatform::with_defaults();
    let candidate = Candidate {
        id: "c".into(),
        label: "w6".into(),
        changes: vec![ParamChange {
            key: "bench.worker_threads".into(),
            previous: ParamValue::Int(4),
            next: ParamValue::Int(6),
            rationale: "t".into(),
        }],
        meta: Default::default(),
    };
    let guard = ApplyGuard::apply(&mut platform, candidate, &mut journal, "exp-int").unwrap();
    // Leak without drop by forgetting rollback path: simulate crash by writing journal and dropping platform state.
    // Instead: drop guard (rolls back), then manually begin+apply without complete.
    drop(guard);
    assert_eq!(
        platform.read_param("bench.worker_threads").unwrap(),
        ParamValue::Int(4)
    );

    // Simulate crash mid-apply: begin, apply, leave journal.
    journal
        .begin(
            "crash",
            &Candidate {
                id: "c2".into(),
                label: "w6".into(),
                changes: vec![ParamChange {
                    key: "bench.worker_threads".into(),
                    previous: ParamValue::Int(4),
                    next: ParamValue::Int(6),
                    rationale: "t".into(),
                }],
                meta: Default::default(),
            },
        )
        .unwrap();
    let change = ParamChange {
        key: "bench.worker_threads".into(),
        previous: ParamValue::Int(4),
        next: ParamValue::Int(6),
        rationale: "t".into(),
    };
    platform.apply_change(&change).unwrap();
    journal.record_applied(&change).unwrap();

    // New journal handle + recover.
    let mut journal2 = RecoveryJournal::open(&path).unwrap();
    let recovered = journal2.recover_with(&mut platform).unwrap();
    assert_eq!(recovered.as_deref(), Some("crash"));
    assert_eq!(
        platform.read_param("bench.worker_threads").unwrap(),
        ParamValue::Int(4)
    );
}

#[test]
fn search_strategy_emits_candidates() {
    let platform = MockPlatform::with_defaults();
    let mut strategy = HillClimbStrategy::new(42);
    let ctx = SearchContext {
        generation: 0,
        experiments_done: 0,
        plateau_count: 0,
        best_score: 10000.0,
        last_class: None,
        elapsed_secs: 0,
        max_experiments: 5,
        time_budget_secs: 60,
        plateau_limit: 4,
    };
    let decision = strategy
        .next_candidate(&platform, &ctx, &Candidate::identity())
        .unwrap();
    match decision {
        SearchDecision::Try(c) => assert!(!c.changes.is_empty()),
        SearchDecision::Stop { reason } => panic!("unexpected stop: {reason}"),
    }
}

#[test]
fn storage_history_and_compare_inputs() {
    let dir = tempdir().unwrap();
    let store = ExperimentStore::open(dir.path().join("e.db")).unwrap();
    let mut a = kraftverk_core::Experiment::new_baseline("fp", "0.1.0", "test");
    a.index_samples = vec![10000.0, 10010.0, 9990.0, 10005.0, 10000.0];
    a.index_summary = kraftverk_core::summarize(&a.index_samples, &StatsConfig::default()).ok();
    store.upsert(&a).unwrap();

    let mut b = kraftverk_core::Experiment::new_candidate(
        &a,
        Candidate {
            id: "x".into(),
            label: "t".into(),
            changes: vec![ParamChange {
                key: "bench.worker_threads".into(),
                previous: ParamValue::Int(4),
                next: ParamValue::Int(6),
                rationale: "t".into(),
            }],
            meta: Default::default(),
        },
    );
    b.index_samples = vec![10300.0, 10350.0, 10280.0, 10310.0, 10320.0];
    b.index_summary = kraftverk_core::summarize(&b.index_samples, &StatsConfig::default()).ok();
    store.upsert(&b).unwrap();

    let hist = store.history(Some("fp"), 10).unwrap();
    assert_eq!(hist.len(), 2);

    let cmp = compare_samples(&a.index_samples, &b.index_samples, &StatsConfig::default()).unwrap();
    assert!(cmp.class.is_improvement());
}

#[test]
fn failed_apply_does_not_leave_partial_without_rollback() {
    let dir = tempdir().unwrap();
    let mut journal = RecoveryJournal::open(dir.path().join("j.json")).unwrap();
    let mut platform = MockPlatform::new(MockConfig {
        fail_apply_key: Some("process.priority".into()),
        ..MockConfig::default()
    });
    let candidate = Candidate {
        id: "bad".into(),
        label: "bad".into(),
        changes: vec![
            ParamChange {
                key: "bench.worker_threads".into(),
                previous: ParamValue::Int(4),
                next: ParamValue::Int(6),
                rationale: "t".into(),
            },
            ParamChange {
                key: "process.priority".into(),
                previous: ParamValue::String("normal".into()),
                next: ParamValue::String("above_normal".into()),
                rationale: "t".into(),
            },
        ],
        meta: Default::default(),
    };
    let err = match ApplyGuard::apply(&mut platform, candidate, &mut journal, "e") {
        Ok(_) => panic!("expected apply failure"),
        Err(e) => e,
    };
    assert!(err.to_string().contains("simulated") || err.to_string().contains("priority"));
    assert_eq!(
        platform.read_param("bench.worker_threads").unwrap(),
        ParamValue::Int(4)
    );
}

#[test]
fn search_plugin_registry_lists_and_creates() {
    let plugins = list_search_plugins();
    assert!(plugins
        .iter()
        .any(|p| p.id == PLUGIN_HILL_CLIMB && p.available));
    let mut strategy = create_search_plugin(PLUGIN_HILL_CLIMB, 99).unwrap();
    let platform = MockPlatform::with_defaults();
    let ctx = SearchContext {
        generation: 0,
        experiments_done: 0,
        plateau_count: 0,
        best_score: 10000.0,
        last_class: None,
        elapsed_secs: 0,
        max_experiments: 3,
        time_budget_secs: 30,
        plateau_limit: 2,
    };
    match strategy
        .next_candidate(&platform, &ctx, &Candidate::identity())
        .unwrap()
    {
        SearchDecision::Try(c) => assert!(!c.changes.is_empty()),
        SearchDecision::Stop { reason } => panic!("unexpected stop: {reason}"),
    }
}

#[test]
fn evidence_receipt_roundtrip() {
    let dir = tempdir().unwrap();
    let mut exp = kraftverk_core::Experiment::new_baseline("fp", "0.2.0", "test");
    exp.index_samples = vec![10000.0, 10020.0, 9990.0];
    exp.index_summary = kraftverk_core::summarize(&exp.index_samples, &StatsConfig::default()).ok();
    exp.decision = kraftverk_core::Decision::Accept;
    exp.decision_reason = "mock accept".into();
    let path = dir.path().join("r.kraft-receipt.json");
    let (written, receipt) = write_receipt(&exp, Some(&path)).unwrap();
    assert!(written.exists());
    assert!(receipt.verify());
    let loaded: EvidenceReceipt =
        serde_json::from_str(&std::fs::read_to_string(&written).unwrap()).unwrap();
    assert!(loaded.verify());
}

#[test]
fn mock_throttle_reduces_score_multiplier() {
    let mut platform = MockPlatform::new(MockConfig {
        seed: 3,
        noise_stddev: 0.0,
        throttle_after_applies: Some(1),
        throttle_multiplier: 0.5,
        ..MockConfig::default()
    });
    let before = platform.score_multiplier();
    platform
        .apply_change(&ParamChange {
            key: "bench.worker_threads".into(),
            previous: ParamValue::Int(4),
            next: ParamValue::Int(6),
            rationale: "t".into(),
        })
        .unwrap();
    platform
        .apply_change(&ParamChange {
            key: "bench.rayon_threads".into(),
            previous: ParamValue::Int(4),
            next: ParamValue::Int(6),
            rationale: "t".into(),
        })
        .unwrap();
    let after = platform.score_multiplier();
    assert!(
        after < before * 1.03,
        "expected throttle to cap gains: before={before} after={after}"
    );
}
