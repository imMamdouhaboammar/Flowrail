use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrecencyConfig {
    pub half_life_secs: f64,
    pub max_history_per_item: usize,
    pub max_bonus: f64,
}

impl Default for FrecencyConfig {
    fn default() -> Self {
        Self {
            half_life_secs: 300.0,
            max_history_per_item: 20,
            max_bonus: 250.0,
        }
    }
}

pub struct FrecencyStore {
    history: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    config: FrecencyConfig,
}

impl Default for FrecencyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FrecencyStore {
    pub fn new() -> Self {
        Self::with_config(FrecencyConfig::default())
    }

    pub fn with_config(config: FrecencyConfig) -> Self {
        Self {
            history: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub fn record_access(&self, key: &str) {
        let now = Instant::now();
        let mut guard = self.history.write().unwrap();
        let entries = guard.entry(key.to_string()).or_default();
        entries.push(now);
        if entries.len() > self.config.max_history_per_item {
            entries.remove(0);
        }
    }

    pub fn get_score(&self, key: &str) -> f64 {
        let guard = self.history.read().unwrap();
        let entries = match guard.get(key) {
            Some(e) if !e.is_empty() => e,
            _ => return 0.0,
        };

        let now = Instant::now();
        let lambda = 2.0_f64.ln() / self.config.half_life_secs.max(1.0);

        let mut raw_score = 0.0;
        for &t in entries {
            let elapsed_secs = now.duration_since(t).as_secs_f64();
            let weight = (-lambda * elapsed_secs).exp();
            raw_score += weight;
        }

        // Scale raw score: 1 recent access ~ 50 bonus, 5 accesses ~ 200 bonus, capped at max_bonus
        (raw_score * 50.0).min(self.config.max_bonus)
    }

    pub fn clear(&self) {
        let mut guard = self.history.write().unwrap();
        guard.clear();
    }
}
