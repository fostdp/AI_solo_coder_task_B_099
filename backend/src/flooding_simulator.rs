use crate::alarm_ws::AlarmCommand;
use crate::clickhouse_client::ClickHouseClient;
use crate::metrics;
use crate::models::*;
use tokio::sync::{mpsc, oneshot};

pub struct ShipHydrostatics {
    config: ShipConfig,
    params: DamageParams,
}

impl ShipHydrostatics {
    pub fn new(config: ShipConfig, params: DamageParams) -> Self {
        Self { config, params }
    }

    pub fn calculate_waterplane_area(&self, draft: f64) -> f64 {
        let l = self.config.length_overall;
        let b = self.config.beam;
        let cb = self.params.waterplane_coefficient_base
            + self.params.waterplane_coefficient_draft_factor * (draft / self.config.depth);
        l * b * cb
    }

    pub fn calculate_displacement(&self, draft: f64) -> f64 {
        let l = self.config.length_overall;
        let b = self.config.beam;
        let cb = self.params.block_coefficient_base
            + self.params.block_coefficient_draft_factor * (draft / self.config.depth);
        self.params.sea_water_density * l * b * draft * cb
    }

    pub fn calculate_buoyancy_force(&self, draft: f64) -> f64 {
        self.calculate_displacement(draft) * self.params.gravity
    }

    pub fn calculate_longitudinal_center_of_buoyancy(&self, draft: f64) -> f64 {
        self.config.length_overall * (0.5 + 0.02 * (draft / self.config.depth))
    }

    pub fn calculate_vertical_center_of_buoyancy(&self, draft: f64) -> f64 {
        draft * 0.55
    }

    pub fn calculate_metacentric_radius_bm(&self, draft: f64) -> f64 {
        let l = self.config.length_overall;
        let b = self.config.beam;
        let displacement = self.calculate_displacement(draft) / self.params.sea_water_density;
        let moment_of_inertia = l * b.powi(3) / 12.0;
        moment_of_inertia / displacement
    }

    pub fn calculate_free_surface_correction_tank_method(
        &self,
        flooded_compartments: &[u8],
        damage_severity: f64,
        draft: f64,
    ) -> f64 {
        let displacement_volume = self.calculate_displacement(draft) / self.params.sea_water_density;
        if displacement_volume <= 0.0 {
            return 0.0;
        }

        let permeability = self.params.permeability;
        let fill_ratio = damage_severity.clamp(0.0, 1.0);

        let mut total_free_surface_inertia = 0.0;
        for &compartment_id in flooded_compartments {
            let idx = compartment_id as usize;
            if idx >= self.config.compartment_lengths.len()
                || idx >= self.config.compartment_volumes.len()
            {
                continue;
            }

            let tank_length = self.config.compartment_lengths[idx];
            let tank_volume = self.config.compartment_volumes[idx];
            if tank_length <= 0.0 || self.config.depth <= 0.0 {
                continue;
            }

            let tank_beam = (tank_volume / (tank_length * self.config.depth * permeability))
                .min(self.config.beam);

            let surface_inertia = tank_length * tank_beam.powi(3) / 12.0;

            let surface_factor = (4.0 * fill_ratio * (1.0 - fill_ratio)).max(0.0);

            total_free_surface_inertia += surface_inertia * surface_factor;
        }

        total_free_surface_inertia / displacement_volume
    }

    pub fn calculate_metacentric_height_gm(
        &self,
        draft: f64,
        kg: f64,
        flooded_compartments: &[u8],
        damage_severity: f64,
    ) -> f64 {
        let kb = self.calculate_vertical_center_of_buoyancy(draft);
        let bm = self.calculate_metacentric_radius_bm(draft);
        let km = kb + bm;

        let free_surface_correction = self.calculate_free_surface_correction_tank_method(
            flooded_compartments,
            damage_severity,
            draft,
        );

        km - kg - free_surface_correction
    }

    pub fn calculate_righting_arm(
        &self,
        heel_angle: f64,
        gm: f64,
        _draft: f64,
    ) -> f64 {
        let heel_rad = heel_angle.to_radians();
        let gz = gm * heel_rad.sin();

        let reduction_factor = if heel_angle > 30.0 {
            (1.0 - (heel_angle - 30.0) / 15.0).max(0.3)
        } else {
            1.0
        };

        gz * reduction_factor
    }

    pub fn calculate_righting_moment(&self, gz: f64, displacement: f64) -> f64 {
        displacement * self.params.gravity * gz
    }

    pub fn generate_stability_curve(
        &self,
        draft: f64,
        kg: f64,
        flooded_compartments: &[u8],
        damage_severity: f64,
    ) -> Vec<StabilityPoint> {
        let gm = self
            .calculate_metacentric_height_gm(draft, kg, flooded_compartments, damage_severity);
        let displacement = self.calculate_displacement(draft);

        (0..=90)
            .step_by(1)
            .map(|angle| {
                let heel_angle = angle as f64;
                let gz = self.calculate_righting_arm(heel_angle, gm, draft);
                let moment = self.calculate_righting_moment(gz, displacement);
                StabilityPoint {
                    heel_angle,
                    righting_arm: gz,
                    righting_moment: moment,
                }
            })
            .collect()
    }

    pub fn calculate_range_of_stability(&self, curve: &[StabilityPoint]) -> f64 {
        curve
            .iter()
            .take_while(|p| p.righting_arm > 0.0)
            .last()
            .map(|p| p.heel_angle)
            .unwrap_or(0.0)
    }

    pub fn calculate_max_righting_arm(&self, curve: &[StabilityPoint]) -> f64 {
        curve
            .iter()
            .map(|p| p.righting_arm)
            .fold(f64::NEG_INFINITY, f64::max)
    }

    pub fn calculate_equilibrium_draft(
        &self,
        flooded_compartments: &[u8],
        damage_severity: f64,
    ) -> f64 {
        let initial_draft = self.config.design_draft;
        let flooded_volume: f64 = flooded_compartments
            .iter()
            .map(|&id| {
                let idx = id as usize;
                if idx < self.config.compartment_volumes.len() {
                    self.config.compartment_volumes[idx] * damage_severity
                } else {
                    0.0
                }
            })
            .sum();

        let waterplane_area = self.calculate_waterplane_area(initial_draft);
        let additional_draft = flooded_volume / waterplane_area;

        (initial_draft + additional_draft).min(self.config.depth * self.params.max_safe_draft_depth_ratio)
    }

    pub fn calculate_heel_moment(
        &self,
        flooded_compartments: &[u8],
        damage_severity: f64,
        _draft: f64,
    ) -> f64 {
        let beam = self.config.beam;
        let mut heel_moment = 0.0;

        for &compartment_id in flooded_compartments {
            let idx = compartment_id as usize;
            if idx >= self.config.compartment_volumes.len() {
                continue;
            }

            let volume = self.config.compartment_volumes[idx] * damage_severity;
            let lateral_offset = if idx % 2 == 0 {
                beam * 0.15
            } else {
                -beam * 0.15
            };

            heel_moment += volume * self.params.sea_water_density * self.params.gravity * lateral_offset;
        }

        heel_moment
    }

    pub fn calculate_equilibrium_heel(
        &self,
        heel_moment: f64,
        curve: &[StabilityPoint],
        _displacement: f64,
    ) -> f64 {
        for point in curve {
            let restoring_moment = point.righting_moment;
            if restoring_moment.abs() >= heel_moment.abs() {
                return point.heel_angle * heel_moment.signum();
            }
        }
        90.0 * heel_moment.signum()
    }

    pub fn calculate_trim_angle(
        &self,
        flooded_compartments: &[u8],
        damage_severity: f64,
        draft: f64,
    ) -> f64 {
        let l = self.config.length_overall;
        let mut trim_moment = 0.0;

        for &compartment_id in flooded_compartments {
            let idx = compartment_id as usize;
            if idx >= self.config.compartment_volumes.len() {
                continue;
            }

            let volume = self.config.compartment_volumes[idx] * damage_severity;
            let longitudinal_pos = if idx == 0 {
                l * 0.1
            } else if idx == self.config.compartment_count as usize - 1 {
                l * 0.9
            } else {
                l * (0.2 + idx as f64 * 0.6 / self.config.compartment_count as f64)
            };

            let lcb = self.calculate_longitudinal_center_of_buoyancy(draft);
            let moment_arm = longitudinal_pos - lcb;
            trim_moment += volume * self.params.sea_water_density * self.params.gravity * moment_arm;
        }

        let mtc = self.calculate_moment_to_change_trim(draft);
        if mtc.abs() > 1e-6 {
            trim_moment / mtc
        } else {
            0.0
        }
    }

    pub fn calculate_moment_to_change_trim(&self, draft: f64) -> f64 {
        let l = self.config.length_overall;
        let displacement = self.calculate_displacement(draft) / self.params.sea_water_density;
        let bml = l.powi(3) * self.config.beam / (12.0 * displacement);
        (displacement * self.params.sea_water_density * self.params.gravity * bml) / (100.0 * l)
    }

    pub fn calculate_reserve_buoyancy(&self, draft: f64) -> f64 {
        let total_volume =
            self.config.length_overall * self.config.beam * self.config.depth * self.params.hull_form_factor;
        let displaced_volume = self.calculate_displacement(draft) / self.params.sea_water_density;
        ((total_volume - displaced_volume) / total_volume * 100.0).max(0.0)
    }

    pub fn calculate_sinking_time(
        &self,
        flooded_compartments: &[u8],
        damage_severity: f64,
        draft: f64,
    ) -> f64 {
        let total_vol: f64 = flooded_compartments
            .iter()
            .map(|&id| {
                let idx = id as usize;
                if idx < self.config.compartment_volumes.len() {
                    self.config.compartment_volumes[idx]
                } else {
                    0.0
                }
            })
            .sum();

        let damage_area = damage_severity * self.params.damage_orifice_area_coefficient;
        let head_pressure = (draft - 0.5).max(0.5);
        let flow_rate = damage_area * (2.0 * self.params.gravity * head_pressure).sqrt();

        if flow_rate > 0.0 {
            total_vol / flow_rate
        } else {
            f64::INFINITY
        }
    }

    pub fn assess_safety(&self, gm: f64, heel_angle: f64, reserve_buoyancy: f64) -> bool {
        gm > self.params.min_metacentric_height
            && heel_angle.abs() < self.params.max_safe_heel_angle
            && reserve_buoyancy > self.params.min_reserve_buoyancy
    }

    pub fn simulate_damage(&self, scenario: &FloodingScenario) -> StabilityResult {
        let flooded_compartments = scenario.flooded_compartments.clone();
        let damage_severity = scenario.damage_severity;

        let draft = self.calculate_equilibrium_draft(&flooded_compartments, damage_severity);
        let kg = self.config.depth * 0.5;

        let gm = self
            .calculate_metacentric_height_gm(draft, kg, &flooded_compartments, damage_severity);
        let stability_curve = self
            .generate_stability_curve(draft, kg, &flooded_compartments, damage_severity);
        let displacement = self.calculate_displacement(draft);

        let heel_moment = self.calculate_heel_moment(&flooded_compartments, damage_severity, draft);
        let heel_angle = self.calculate_equilibrium_heel(heel_moment, &stability_curve, displacement);
        let trim_angle = self.calculate_trim_angle(&flooded_compartments, damage_severity, draft);

        let range_of_stability = self.calculate_range_of_stability(&stability_curve);
        let righting_arm_max = self.calculate_max_righting_arm(&stability_curve);
        let reserve_buoyancy = self.calculate_reserve_buoyancy(draft);
        let sinking_time = self.calculate_sinking_time(&flooded_compartments, damage_severity, draft);
        let is_safe = self.assess_safety(gm, heel_angle, reserve_buoyancy);

        StabilityResult {
            simulation_id: uuid::Uuid::new_v4(),
            ship_id: self.config.ship_id.clone(),
            timestamp: chrono::Utc::now(),
            flooded_compartments,
            final_draft: draft,
            final_heel_angle: heel_angle,
            final_trim_angle: trim_angle,
            metacentric_height: gm,
            righting_arm_max,
            range_of_stability,
            is_safe,
            sinking_time_seconds: sinking_time,
            reserve_buoyancy,
            stability_curve,
        }
    }

    pub fn calculate_max_floodable_compartments(&self) -> u8 {
        for n in (1..=self.config.compartment_count).rev() {
            let compartments: Vec<u8> = (0..n).collect();
            let scenario = FloodingScenario {
                ship_id: self.config.ship_id.clone(),
                flooded_compartments: compartments,
                damage_severity: 0.5,
            };
            let result = self.simulate_damage(&scenario);
            if result.is_safe {
                return n;
            }
        }
        0
    }

    pub fn simulate_with_door_states(
        &self,
        scenario: &FloodingScenario,
        door_states: &[crate::models::BulkheadDoorState],
    ) -> StabilityResult {
        let mut effective_flooded = scenario.flooded_compartments.clone();
        let open_doors: std::collections::HashSet<u8> = door_states
            .iter()
            .filter(|d| d.is_open)
            .map(|d| d.bulkhead_id)
            .collect();

        let mut changed = true;
        while changed {
            changed = false;
            let current: std::collections::HashSet<u8> = effective_flooded.iter().cloned().collect();
            for &compartment in &effective_flooded {
                if compartment > 0 && open_doors.contains(&(compartment - 1)) {
                    let left = compartment - 1;
                    if !current.contains(&left) {
                        effective_flooded.push(left);
                        changed = true;
                    }
                }
                let right = compartment + 1;
                if right < self.config.compartment_count && open_doors.contains(&compartment) {
                    if !current.contains(&right) {
                        effective_flooded.push(right);
                        changed = true;
                    }
                }
            }
        }

        effective_flooded.sort();
        effective_flooded.dedup();

        let effective_scenario = FloodingScenario {
            ship_id: scenario.ship_id.clone(),
            flooded_compartments: effective_flooded,
            damage_severity: scenario.damage_severity,
        };

        self.simulate_damage(&effective_scenario)
    }

    pub fn simulate_pirate_attack(
        &self,
        attack: &crate::models::PirateAttackRequest,
    ) -> crate::models::PirateAttackResult {
        let mut timeline = Vec::new();
        let mut active_compartments: Vec<u8> = Vec::new();
        let mut critical_moments = Vec::new();
        let mut last_gm = f64::MAX;

        let max_time = attack.simulation_duration_seconds.max(600.0);
        let step = 10.0;
        let mut current_time = 0.0;

        timeline.push(crate::models::TimelineEvent {
            time_seconds: 0.0,
            event_type: "START".to_string(),
            description: "船舶正常航行状态".to_string(),
            affected_compartments: Vec::new(),
            gm_value: self.config.depth * 0.15,
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

                    timeline.push(crate::models::TimelineEvent {
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
                let result = self.simulate_damage(&scenario);
                last_gm = result.metacentric_height;

                if (last_gm - 0.15).abs() < 0.02 && result.metacentric_height <= 0.15 {
                    critical_moments.push(crate::models::CriticalMoment {
                        time_seconds: current_time,
                        description: "初稳心高GM降至安全阈值以下".to_string(),
                        gm_value: result.metacentric_height,
                    });
                }

                if !result.is_safe {
                    timeline.push(crate::models::TimelineEvent {
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
                    critical_moments.push(crate::models::CriticalMoment {
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
        let final_result = self.simulate_damage(&final_scenario);

        let survival_probability = if final_result.is_safe {
            (final_result.metacentric_height / 0.5).min(1.0)
                * (final_result.reserve_buoyancy / 30.0).min(1.0)
        } else {
            0.0
        };

        if final_result.is_safe {
            timeline.push(crate::models::TimelineEvent {
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

        crate::models::PirateAttackResult {
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
}

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
        request: crate::models::InteractiveSimulationRequest,
        reply: oneshot::Sender<Result<StabilityResult, String>>,
    },
    SimulatePirateAttack {
        request: crate::models::PirateAttackRequest,
        reply: oneshot::Sender<Result<crate::models::PirateAttackResult, String>>,
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
        request: &crate::models::InteractiveSimulationRequest,
    ) -> Result<StabilityResult, String> {
        metrics::SIMULATIONS_TOTAL.inc();
        let config = self
            .clickhouse
            .get_ship_config(&request.ship_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Ship config not found for id: {}", request.ship_id))?;

        let hydrostatics = ShipHydrostatics::new(config.clone(), self.damage_params.clone());

        let scenario = FloodingScenario {
            ship_id: request.ship_id.clone(),
            flooded_compartments: request.flooded_compartments.clone(),
            damage_severity: request.damage_severity,
        };

        let result = hydrostatics.simulate_with_door_states(&scenario, &request.door_states);

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
        request: &crate::models::PirateAttackRequest,
    ) -> Result<crate::models::PirateAttackResult, String> {
        metrics::SIMULATIONS_TOTAL.inc();
        let config = self
            .clickhouse
            .get_ship_config(&request.ship_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Ship config not found for id: {}", request.ship_id))?;

        let hydrostatics = ShipHydrostatics::new(config.clone(), self.damage_params.clone());
        let result = hydrostatics.simulate_pirate_attack(request);

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
