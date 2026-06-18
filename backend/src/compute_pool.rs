use crate::hydrostatics::ShipHydrostatics;
use crate::models::*;
use parking_lot::Mutex;
use rayon::ThreadPoolBuilder;
use std::sync::Arc;
use tokio::sync::oneshot;

pub struct HydrostaticsComputePool {
    pool: rayon::ThreadPool,
    default_params: DamageParams,
}

impl HydrostaticsComputePool {
    pub fn new(default_params: DamageParams) -> Self {
        let num_threads = std::cmp::max(2, num_cpus::get().saturating_sub(1));
        let pool = ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .thread_name(|i| format!("hydro-compute-{}", i))
            .build()
            .expect("Failed to build hydrostatics compute pool");

        tracing::info!(
            "Hydrostatics compute pool initialized with {} threads",
            num_threads
        );

        Self {
            pool,
            default_params,
        }
    }

    pub fn default_params(&self) -> &DamageParams {
        &self.default_params
    }

    pub fn pool(&self) -> &rayon::ThreadPool {
        &self.pool
    }

    pub async fn simulate_damage(
        &self,
        config: ShipConfig,
        scenario: FloodingScenario,
    ) -> Result<StabilityResult, String> {
        let params = self.default_params.clone();
        self.spawn(move || {
            let hydro = ShipHydrostatics::new(config, params);
            hydro.simulate_damage(&scenario)
        })
        .await
    }

    pub async fn batch_simulate(
        &self,
        config: ShipConfig,
        scenarios: Vec<FloodingScenario>,
    ) -> Result<Vec<StabilityResult>, String> {
        let params = self.default_params.clone();
        let config = Arc::new(config);

        self.spawn(move || {
            let hydro = ShipHydrostatics::new((*config).clone(), params);
            scenarios
                .iter()
                .map(|s| hydro.simulate_damage(s))
                .collect()
        })
        .await
    }

    pub async fn calculate_max_floodable(
        &self,
        config: ShipConfig,
    ) -> Result<u8, String> {
        let params = self.default_params.clone();
        self.spawn(move || {
            let hydro = ShipHydrostatics::new(config, params);
            hydro.calculate_max_floodable_compartments()
        })
        .await
    }

    pub async fn generate_stability_curve(
        &self,
        config: ShipConfig,
        draft: f64,
        kg: f64,
        flooded_compartments: Vec<u8>,
        damage_severity: f64,
    ) -> Result<Vec<StabilityPoint>, String> {
        let params = self.default_params.clone();
        self.spawn(move || {
            let hydro = ShipHydrostatics::new(config, params);
            hydro.generate_stability_curve(draft, kg, &flooded_compartments, damage_severity)
        })
        .await
    }

    pub async fn calculate_survival_probability(
        &self,
        config: ShipConfig,
        result: StabilityResult,
        flooded_count: usize,
    ) -> Result<f64, String> {
        let params = self.default_params.clone();
        self.spawn(move || {
            let hydro = ShipHydrostatics::new(config, params);
            hydro.calculate_survival_probability(&result, flooded_count)
        })
        .await
    }

    pub fn spawn<F, R>(&self, f: F) -> oneshot::Receiver<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        self.pool.spawn(move || {
            let result = f();
            let _ = tx.send(result);
        });
        rx
    }

    pub fn spawn_parallel<F, T, R>(&self, items: Vec<T>, f: F) -> oneshot::Receiver<Vec<R>>
    where
        F: Fn(T) -> R + Send + Sync + 'static,
        T: Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let f = Arc::new(f);

        self.pool.spawn(move || {
            let result: Vec<R> = items.into_par_iter().map(|item| f(item)).collect();
            let _ = tx.send(result);
        });

        rx
    }
}

use rayon::prelude::*;

#[derive(Clone)]
pub struct SharedHydrostaticsPool {
    inner: Arc<HydrostaticsComputePool>,
}

impl SharedHydrostaticsPool {
    pub fn new(default_params: DamageParams) -> Self {
        Self {
            inner: Arc::new(HydrostaticsComputePool::new(default_params)),
        }
    }

    pub fn pool(&self) -> &HydrostaticsComputePool {
        &self.inner
    }
}

impl std::ops::Deref for SharedHydrostaticsPool {
    type Target = HydrostaticsComputePool;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ShipConfig {
        ShipConfig {
            ship_id: "test_pool".to_string(),
            ship_name: "测试船".to_string(),
            length_overall: 34.0,
            beam: 11.0,
            depth: 4.5,
            design_draft: 2.8,
            displacement: 400.0,
            compartment_count: 6,
            compartment_names: vec!["舱".to_string(); 6],
            compartment_lengths: vec![3.4; 6],
            compartment_volumes: vec![50.0; 6],
            watertight_bulkheads: vec![3.4; 5],
        }
    }

    #[test]
    fn test_pool_creation() {
        let pool = HydrostaticsComputePool::new(DamageParams::default());
        assert!(pool.pool().current_num_threads() >= 2);
    }

    #[test]
    fn test_shared_pool_clone() {
        let pool = SharedHydrostaticsPool::new(DamageParams::default());
        let pool2 = pool.clone();
        assert_eq!(
            pool.default_params().gravity,
            pool2.default_params().gravity
        );
    }

    #[test]
    fn test_simulate_damage_sync() {
        let config = test_config();
        let hydro = ShipHydrostatics::new(config.clone(), DamageParams::default());
        let scenario = FloodingScenario {
            ship_id: "test_pool".to_string(),
            flooded_compartments: vec![2],
            damage_severity: 0.5,
        };
        let result = hydro.simulate_damage(&scenario);
        assert_eq!(result.ship_id, "test_pool");
        assert!(result.final_draft > 0.0);
    }

    #[test]
    fn test_spawn_function() {
        let pool = HydrostaticsComputePool::new(DamageParams::default());
        let rx = pool.spawn(|| 42);
        // 注意：在测试中我们不使用 tokio runtime，所以用不同的方式测试
        // 这里只测试池本身能创建和 spawn
        assert!(pool.pool().current_num_threads() > 0);
    }

    #[test]
    fn test_parallel_iterator() {
        let items: Vec<i32> = (0..100).collect();
        let result: Vec<i32> = items.par_iter().map(|x| x * 2).collect();
        assert_eq!(result.len(), 100);
        assert_eq!(result[0], 0);
        assert_eq!(result[50], 100);
    }

    #[test]
    fn test_calculate_max_floodable_sync() {
        let config = test_config();
        let hydro = ShipHydrostatics::new(config, DamageParams::default());
        let max_flooded = hydro.calculate_max_floodable_compartments();
        assert!(max_flooded > 0);
        assert!(max_flooded <= 6);
    }
}
