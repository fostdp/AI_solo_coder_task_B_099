use crate::alarm_ws::AlarmCommand;
use crate::clickhouse_client::ClickHouseClient;
use crate::extreme_simulator::ExtremeFloodingSimulator;
use crate::hydrostatics::ShipHydrostatics;
use crate::metrics;
use crate::models::*;
use crate::vr_compartment::VRCompartmentSimulator;
use tokio::sync::{mpsc, oneshot};

pub use crate::hydrostatics::ShipHydrostatics;

pub enum SimCommand {
    Simulate {
        scenario: FloodingScenario,
        reply: oneshot::Sender<Result<StabilityResult, String>>,
    },
    BatchSimulate {
        scenarios: Vec<FloodingScenario>,
        reply: oneshot::Sender<Result<Vec<StabilityResult>, String>>,
    },
    SimulateInteractive {
        request: InteractiveSimulationRequest,
        reply: oneshot::Sender<Result<StabilityResult, String>>,
    },
    SimulatePirateAttack {
        request: PirateAttackRequest,
        reply: oneshot::Sender<Result<PirateAttackResult, String>>,
    },
}

pub struct FloodingSimulator {
    rx: mpsc::Receiver<SimCommand>,
    alarm_tx: mpsc::Sender<AlarmCommand>,
    clickhouse: ClickHouseClient,
    damage_params: DamageParams,
}

impl FloodingSimulator {
    pub fn new(
        rx: mpsc::Receiver<SimCommand>,
        alarm_tx: mpsc::Sender<AlarmCommand>,
        clickhouse: ClickHouseClient,
        damage_params: DamageParams,
    ) -> Self {
        Self {
            rx,
            alarm_tx,
            clickhouse,
            damage_params,
        }
    }

    pub async fn run(mut self) {
        tracing::info!("FloodingSimulator task started");
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                SimCommand::Simulate { scenario, reply } => {
                    let result = self.handle_simulate(&scenario).await;
                    let _ = reply.send(result);
                }
                SimCommand::BatchSimulate { scenarios, reply } => {
                    let mut results = Vec::with_capacity(scenarios.len());
                    for scenario in &scenarios {
                        if let Ok(r) = self.handle_simulate(scenario).await {
                            results.push(r);
                        }
                    }
                    let _ = reply.send(Ok(results));
                }
                SimCommand::SimulateInteractive { request, reply } => {
                    let result = self.handle_interactive_simulate(&request).await;
                    let _ = reply.send(result);
                }
                SimCommand::SimulatePirateAttack { request, reply } => {
                    let result = self.handle_pirate_attack(&request).await;
                    let _ = reply.send(result);
                }
            }
        }
        tracing::info!("FloodingSimulator task stopped");
    }

    async fn handle_simulate(&self, scenario: &FloodingScenario) -> Result<StabilityResult, String> {
        metrics::SIMULATIONS_TOTAL.inc();
        let config = self
            .clickhouse
            .get_ship_config(&scenario.ship_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Ship config not found for id: {}", scenario.ship_id))?;

        let hydrostatics = ShipHydrostatics::new(config.clone(), self.damage_params.clone());
        let result = hydrostatics.simulate_damage(scenario);

        if let Err(e) = self.clickhouse.insert_simulation_result(&result).await {
            tracing::error!("Failed to insert simulation result: {}", e);
        }

        let _ = self
            .alarm_tx
            .send(AlarmCommand::EvaluateResult {
                result: result.clone(),
                config,
            })
            .await;

        Ok(result)
    }

    async fn handle_interactive_simulate(
        &self,
        request: &InteractiveSimulationRequest,
    ) -> Result<StabilityResult, String> {
        metrics::SIMULATIONS_TOTAL.inc();
        let config = self
            .clickhouse
            .get_ship_config(&request.ship_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Ship config not found for id: {}", request.ship_id))?;

        let hydrostatics = ShipHydrostatics::new(config.clone(), self.damage_params.clone());
        let mut vr_sim = VRCompartmentSimulator::new(hydrostatics);

        for door in &request.door_states {
            let _ = vr_sim.set_door_state(door.bulkhead_id, door.is_open);
        }

        let scenario = FloodingScenario {
            ship_id: request.ship_id.clone(),
            flooded_compartments: request.flooded_compartments.clone(),
            damage_severity: request.damage_severity,
        };

        let vr_result = vr_sim.simulate_with_door_states(&scenario);
        let result = vr_result.stability_result;

        if let Err(e) = self.clickhouse.insert_simulation_result(&result).await {
            tracing::error!("Failed to insert simulation result: {}", e);
        }

        let _ = self
            .alarm_tx
            .send(AlarmCommand::EvaluateResult {
                result: result.clone(),
                config,
            })
            .await;

        Ok(result)
    }

    async fn handle_pirate_attack(
        &self,
        request: &PirateAttackRequest,
    ) -> Result<PirateAttackResult, String> {
        metrics::SIMULATIONS_TOTAL.inc();
        let config = self
            .clickhouse
            .get_ship_config(&request.ship_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Ship config not found for id: {}", request.ship_id))?;

        let hydrostatics = ShipHydrostatics::new(config.clone(), self.damage_params.clone());
        let extreme_sim = ExtremeFloodingSimulator::new(hydrostatics);
        let result = extreme_sim.simulate_pirate_attack(request);

        let _ = self
            .alarm_tx
            .send(AlarmCommand::EvaluateResult {
                result: result.final_state.clone(),
                config,
            })
            .await;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;

    fn test_ship_config() -> ShipConfig {
        let count = 10u8;
        let length = 34.0;
        let compartment_lengths: Vec<f64> = (0..count).map(|_| length / count as f64).collect();
        let compartment_volumes: Vec<f64> = (0..count)
            .map(|_| (length / count as f64) * 8.0 * 2.5)
            .collect();
        let compartment_names: Vec<String> = (0..count)
            .map(|i| format!("舱室{}", i + 1))
            .collect();
        let watertight_bulkheads: Vec<f64> = (1..count)
            .map(|i| (i as f64) * length / count as f64)
            .collect();

        ShipConfig {
            ship_id: "test_ship_001".to_string(),
            ship_name: "测试船".to_string(),
            length_overall: length,
            beam: 8.0,
            depth: 4.5,
            design_draft: 2.8,
            displacement: 400.0,
            compartment_count: count,
            compartment_names,
            compartment_lengths,
            compartment_volumes,
            watertight_bulkheads,
        }
    }

    fn test_hydrostatics() -> ShipHydrostatics {
        ShipHydrostatics::new(test_ship_config(), DamageParams::default())
    }

    #[test]
    fn test_sim_command_variants() {
        let (tx, _rx) = oneshot::channel();
        let cmd = SimCommand::Simulate {
            scenario: FloodingScenario {
                ship_id: "test".to_string(),
                flooded_compartments: vec![],
                damage_severity: 0.0,
            },
            reply: tx,
        };
        match cmd {
            SimCommand::Simulate { .. } => {}
            _ => panic!("应该是 Simulate 变体"),
        }
    }

    #[test]
    fn test_sim_command_interactive_variant() {
        let (tx, _rx) = oneshot::channel();
        let cmd = SimCommand::SimulateInteractive {
            request: InteractiveSimulationRequest {
                ship_id: "test".to_string(),
                flooded_compartments: vec![],
                damage_severity: 0.0,
                door_states: vec![],
            },
            reply: tx,
        };
        match cmd {
            SimCommand::SimulateInteractive { .. } => {}
            _ => panic!("应该是 SimulateInteractive 变体"),
        }
    }

    #[test]
    fn test_sim_command_pirate_variant() {
        let (tx, _rx) = oneshot::channel();
        let cmd = SimCommand::SimulatePirateAttack {
            request: PirateAttackRequest {
                ship_id: "test".to_string(),
                attack_points: vec![],
                simulation_duration_seconds: 600.0,
            },
            reply: tx,
        };
        match cmd {
            SimCommand::SimulatePirateAttack { .. } => {}
            _ => panic!("应该是 SimulatePirateAttack 变体"),
        }
    }

    #[test]
    fn test_ship_hydrostatics_reexport_works() {
        let hydro = test_hydrostatics();
        assert_eq!(hydro.config().ship_id, "test_ship_001");
    }

    #[test]
    fn test_flooding_simulator_struct_size() {
        use std::mem::size_of;
        assert!(size_of::<SimCommand>() > 0);
        assert!(size_of::<FloodingSimulator>() > 0);
    }
}
