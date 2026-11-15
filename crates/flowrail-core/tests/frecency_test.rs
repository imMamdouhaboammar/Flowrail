use flowrail_core::frecency::{FrecencyConfig, FrecencyStore};

#[test]
fn test_frecency_store_records_and_scores() {
    let store = FrecencyStore::new();

    assert_eq!(store.get_score("unknown"), 0.0);

    store.record_access("sym1");
    let score1 = store.get_score("sym1");
    assert!(score1 > 40.0);

    store.record_access("sym1");
    let score2 = store.get_score("sym1");
    assert!(score2 > score1);

    store.clear();
    assert_eq!(store.get_score("sym1"), 0.0);
}

#[test]
fn test_frecency_capped_at_max() {
    let config = FrecencyConfig {
        half_life_secs: 100.0,
        max_history_per_item: 10,
        max_bonus: 150.0,
    };
    let store = FrecencyStore::with_config(config);

    for _ in 0..10 {
        store.record_access("hot_symbol");
    }

    assert!(store.get_score("hot_symbol") <= 150.0);
}
