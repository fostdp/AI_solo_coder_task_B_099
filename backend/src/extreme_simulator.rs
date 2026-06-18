use crate::hydrostatics::ShipHydrostatics;
use crate::models::*;

pub struct ExtremeFloodingSimulator {
    hydrostatics: ShipHydrostatics,
}

impl ExtremeFloodingSimulator {
    pub fn new(hydrostatics: ShipHydrostatics) -> Self {
        Self { hydrostatics }
    }

    pub fn hydrostatics(&self) -> &ShipHydrostatics {
        &self.hydrostatics
    }

    pub fn simulate_pirate_attack(
        &self,
        attack: &PirateAttackRequest,
    ) -> PirateAttackResult {
        let mut timeline = Vec::new();
        let mut active_compartments: Vec<u8> = Vec::new();
        let mut critical_moments = Vec::new();
        let mut last_gm = f64::MAX;

        let max_time = attack.simulation_duration_seconds.max(600.0);
        let step = 10.0;
        let mut current_time = 0.0;

        timeline.push(TimelineEvent {
            time_seconds: 0.0,
            event_type: "START".to_string(),
            description: "船舶正常航行状态".to_string(),
            affected_compartments: Vec::new(),
            gm_value: self.hydrostatics.config().depth * 0.15,
            is_safe: true,
        });

        let mut attack_iter = attack.attack_points.iter().peekable();

        while current_time <= max_time {
            while let Some(ap) = attack_iter.peek() {
                if ap.delay_seconds <= current_time {
                    let ap = attack_iter.next().unwrap();
                    active_compartments.push(ap.compartment_id);
                    active_compartments.sort();
                    active_compartments.dedup();

                    timeline.push(TimelineEvent {
                        time_seconds: current_time,
                        event_type: "ATTACK".to_string(),
                        description: format!(
                            "海盗攻击！舱室{}破损，严重度{:.1}",
                            ap.compartment_id, ap.damage_severity
                        ),
                        affected_compartments: active_compartments.clone(),
                        gm_value: last_gm,
                        is_safe: true,
                    });
                } else {
                    break;
                }
            }

            if !active_compartments.is_empty() {
                let scenario = FloodingScenario {
                    ship_id: attack.ship_id.clone(),
                    flooded_compartments: active_compartments.clone(),
                    damage_severity: 0.8,
                };
                let result = self.hydrostatics.simulate_damage(&scenario);
                last_gm = result.metacentric_height;

                if (last_gm - 0.15).abs() < 0.02 && result.metacentric_height <= 0.15 {
                    critical_moments.push(CriticalMoment {
                        time_seconds: current_time,
                        description: "初稳心高GM降至安全阈值以下".to_string(),
                        gm_value: result.metacentric_height,
                    });
                }

                if !result.is_safe {
                    timeline.push(TimelineEvent {
                        time_seconds: current_time,
                        event_type: "UNSAFE".to_string(),
                        description: format!(
                            "船舶状态转为危险！GM={:.3}m, 横倾={:.1}°, 储备浮力={:.1}%",
                            result.metacentric_height,
                            result.final_heel_angle,
                            result.reserve_buoyancy
                        ),
                        affected_compartments: active_compartments.clone(),
                        gm_value: result.metacentric_height,
                        is_safe: false,
                    });
                    break;
                }

                if result.sinking_time_seconds < 600.0 && result.sinking_time_seconds > 0.0 {
                    critical_moments.push(CriticalMoment {
                        time_seconds: current_time,
                        description: format!(
                            "预计下沉时间不足10分钟！下沉时间估算: {:.0}秒",
                            result.sinking_time_seconds
                        ),
                        gm_value: result.metacentric_height,
                    });
                }
            }

            current_time += step;
        }

        let final_scenario = FloodingScenario {
            ship_id: attack.ship_id.clone(),
            flooded_compartments: active_compartments.clone(),
            damage_severity: 0.9,
        };
        let final_result = self.hydrostatics.simulate_damage(&final_scenario);

        let survival_probability = self.hydrostatics.calculate_survival_probability(
            &final_result,
            active_compartments.len(),
        );

        if final_result.is_safe {
            timeline.push(TimelineEvent {
                time_seconds: current_time,
                event_type: "SURVIVED".to_string(),
                description: format!(
                    "海盗攻击结束，船舶成功生存！最终进水{}舱，GM={:.3}m",
                    active_compartments.len(),
                    final_result.metacentric_height
                ),
                affected_compartments: active_compartments.clone(),
                gm_value: final_result.metacentric_height,
                is_safe: true,
            });
        }

        PirateAttackResult {
            simulation_id: uuid::Uuid::new_v4(),
            ship_id: attack.ship_id.clone(),
            timestamp: chrono::Utc::now(),
            attack_points: attack.attack_points.clone(),
            timeline_events: timeline,
            final_state: final_result,
            survival_probability,
            critical_moments,
        }
    }

    pub fn simulate_capsize_scenario(
        &self,
        wind_speed: f64,
        flooded_compartments: &[u8],
    ) -> CapsizeResult {
        let scenario = FloodingScenario {
            ship_id: self.hydrostatics.config().ship_id.clone(),
            flooded_compartments: flooded_compartments.to_vec(),
            damage_severity: 0.8,
        };
        let base_result = self.hydrostatics.simulate_damage(&scenario);

        let wind_heeling_moment = wind_speed.powi(2) * self.hydrostatics.config().length_overall
            * self.hydrostatics.config().beam
            * 0.001;

        let displacement = self.hydrostatics.calculate_displacement(base_result.final_draft);
        let gm = base_result.metacentric_height;
        let wind_heel_angle = (wind_heeling_moment
            / (displacement * self.hydrostatics.params().gravity * gm.max(0.01)))
            .to_degrees();

        let will_capsize = wind_heel_angle > base_result.range_of_stability;

        CapsizeResult {
            simulation_id: uuid::Uuid::new_v4(),
            ship_id: self.hydrostatics.config().ship_id.clone(),
            timestamp: chrono::Utc::now(),
            wind_speed,
            wind_heel_angle,
            range_of_stability: base_result.range_of_stability,
            will_capsize,
            flooded_compartments: flooded_compartments.to_vec(),
            base_stability: base_result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DamageParams;

    fn test_config() -> ShipConfig {
        ShipConfig {
            ship_id: "test_ship".to_string(),
            ship_name: "测试船".to_string(),
            length_overall: 34.0,
            beam: 11.0,
            depth: 4.5,
            design_draft: 2.8,
            displacement: 400.0,
            compartment_count: 10,
            compartment_names: vec!["舱".to_string(); 10],
            compartment_lengths: vec![3.4; 10],
            compartment_volumes: vec![50.0; 10],
            watertight_bulkheads: vec![3.4; 9],
        }
    }

    fn test_simulator() -> ExtremeFloodingSimulator {
        let hydro = ShipHydrostatics::new(test_config(), DamageParams::default());
        ExtremeFloodingSimulator::new(hydro)
    }

    #[test]
    fn test_extreme_simulator_new() {
        let sim = test_simulator();
        assert_eq!(sim.hydrostatics().config().ship_id, "test_ship");
    }

    #[test]
    fn test_pirate_attack_single() {
        let sim = test_simulator();
        let attack = PirateAttackRequest {
            ship_id: "test_ship".to_string(),
            attack_points: vec![AttackPoint {
                compartment_id: 2,
                damage_severity: 0.3,
                delay_seconds: 0.0,
            }],
            simulation_duration_seconds: 600.0,
        };
        let result = sim.simulate_pirate_attack(&attack);
        assert_eq!(result.ship_id, "test_ship");
        assert!(!result.timeline_events.is_empty());
        assert!(result.survival_probability > 0.0);
        assert!(result.survival_probability <= 1.0);
    }

    #[test]
    fn test_pirate_attack_survival_probability_range() {
        let sim = test_simulator();
        let mild = PirateAttackRequest {
            ship_id: "test_ship".to_string(),
            attack_points: vec![AttackPoint {
                compartment_id: 2,
                damage_severity: 0.3,
                delay_seconds: 0.0,
            }],
            simulation_duration_seconds: 600.0,
        };
        let mild_result = sim.simulate_pirate_attack(&mild);
        assert!(mild_result.survival_probability > 0.5);
    }

    #[test]
    fn test_pirate_multiple_attacks() {
        let sim = test_simulator();
        let attack = PirateAttackRequest {
            ship_id: "test_ship".to_string(),
            attack_points: vec![
                AttackPoint { compartment_id: 2, damage_severity: 0.5, delay_seconds: 0.0 },
                AttackPoint { compartment_id: 3, damage_severity: 0.5, delay_seconds: 100.0 },
                AttackPoint { compartment_id: 4, damage_severity: 0.5, delay_seconds: 200.0 },
            ],
            simulation_duration_seconds: 600.0,
        };
        let result = sim.simulate_pirate_attack(&attack);
        assert!(result.timeline_events.len() >= 4);
        let attack_events: Vec<_> = result.timeline_events.iter()
            .filter(|e| e.event_type == "ATTACK")
            .collect();
        assert_eq!(attack_events.len(), 3);
    }

    #[test]
    fn test_capsize_scenario_calm() {
        let sim = test_simulator();
        let result = sim.simulate_capsize_scenario(5.0, &[2]);
        assert!(!result.will_capsize);
        assert!(result.wind_heel_angle >= 0.0);
    }

    #[test]
    fn test_capsize_scenario_storm() {
        let sim = test_simulator();
        let result = sim.simulate_capsize_scenario(50.0, &[2, 3, 4, 5]);
        assert!(result.wind_speed == 50.0);
        assert!(result.range_of_stability > 0.0);
    }

    #[test]
    fn test_pirate_attack_timeline_starts_with_start() {
        let sim = test_simulator();
        let attack = PirateAttackRequest {
            ship_id: "test".to_string(),
            attack_points: vec![],
            simulation_duration_seconds: 100.0,
        };
        let result = sim.simulate_pirate_attack(&attack);
        assert_eq!(result.timeline_events[0].event_type, "START");
    }

    #[test]
    fn test_pirate_attack_critical_moments() {
        let sim = test_simulator();
        let attack = PirateAttackRequest {
            ship_id: "test_ship".to_string(),
            attack_points: vec![
                AttackPoint { compartment_id: 1, damage_severity: 0.9, delay_seconds: 0.0 },
                AttackPoint { compartment_id: 2, damage_severity: 0.9, delay_seconds: 10.0 },
                AttackPoint { compartment_id: 3, damage_severity: 0.9, delay_seconds: 20.0 },
                AttackPoint { compartment_id: 4, damage_severity: 0.9, delay_seconds: 30.0 },
                AttackPoint { compartment_id: 5, damage_severity: 0.9, delay_seconds: 40.0 },
            ],
            simulation_duration_seconds: 600.0,
        };
        let result = sim.simulate_pirate_attack(&attack);
        assert!(!result.critical_moments.is_empty() || !result.final_state.is_safe);
    }
}
