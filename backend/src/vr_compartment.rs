use crate::hydrostatics::ShipHydrostatics;
use crate::models::*;
use std::collections::HashSet;

pub struct VRCompartmentSimulator {
    hydrostatics: ShipHydrostatics,
    door_states: Vec<BulkheadDoorState>,
}

impl VRCompartmentSimulator {
    pub fn new(hydrostatics: ShipHydrostatics) -> Self {
        let compartment_count = hydrostatics.config().compartment_count as usize;
        let door_states = (0..compartment_count.saturating_sub(1))
            .map(|i| BulkheadDoorState {
                bulkhead_id: i as u8,
                is_open: false,
                last_changed: chrono::Utc::now(),
            })
            .collect();

        Self {
            hydrostatics,
            door_states,
        }
    }

    pub fn hydrostatics(&self) -> &ShipHydrostatics {
        &self.hydrostatics
    }

    pub fn door_states(&self) -> &[BulkheadDoorState] {
        &self.door_states
    }

    pub fn set_door_state(&mut self, bulkhead_id: u8, is_open: bool) -> Result<(), String> {
        let idx = bulkhead_id as usize;
        if idx >= self.door_states.len() {
            return Err(format!(
                "Invalid bulkhead_id: {}, max is {}",
                bulkhead_id,
                self.door_states.len().saturating_sub(1)
            ));
        }

        self.door_states[idx] = BulkheadDoorState {
            bulkhead_id,
            is_open,
            last_changed: chrono::Utc::now(),
        };

        Ok(())
    }

    pub fn open_door(&mut self, bulkhead_id: u8) -> Result<(), String> {
        self.set_door_state(bulkhead_id, true)
    }

    pub fn close_door(&mut self, bulkhead_id: u8) -> Result<(), String> {
        self.set_door_state(bulkhead_id, false)
    }

    pub fn close_all_doors(&mut self) {
        for door in &mut self.door_states {
            door.is_open = false;
            door.last_changed = chrono::Utc::now();
        }
    }

    pub fn open_all_doors(&mut self) {
        for door in &mut self.door_states {
            door.is_open = true;
            door.last_changed = chrono::Utc::now();
        }
    }

    pub fn simulate_with_door_states(
        &self,
        scenario: &FloodingScenario,
    ) -> VRSimulationResult {
        let mut effective_flooded = scenario.flooded_compartments.clone();
        let open_doors: HashSet<u8> = self
            .door_states
            .iter()
            .filter(|d| d.is_open)
            .map(|d| d.bulkhead_id)
            .collect();

        let mut changed = true;
        while changed {
            changed = false;
            let current: HashSet<u8> = effective_flooded.iter().cloned().collect();
            for &compartment in &effective_flooded {
                if compartment > 0 && open_doors.contains(&(compartment - 1)) {
                    let left = compartment - 1;
                    if !current.contains(&left) {
                        effective_flooded.push(left);
                        changed = true;
                    }
                }
                let right = compartment + 1;
                if right < self.hydrostatics.config().compartment_count
                    && open_doors.contains(&compartment)
                {
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
            flooded_compartments: effective_flooded.clone(),
            damage_severity: scenario.damage_severity,
        };

        let stability_result = self.hydrostatics.simulate_damage(&effective_scenario);

        let spread_chains = self.calculate_spread_chains(&scenario.flooded_compartments, &effective_flooded);

        VRSimulationResult {
            simulation_id: uuid::Uuid::new_v4(),
            ship_id: self.hydrostatics.config().ship_id.clone(),
            timestamp: chrono::Utc::now(),
            initial_damage: scenario.flooded_compartments.clone(),
            effective_flooded: effective_flooded.clone(),
            damage_severity: scenario.damage_severity,
            door_states: self.door_states.clone(),
            spread_through_doors: effective_flooded.len() > scenario.flooded_compartments.len(),
            spread_chains,
            stability_result,
        }
    }

    fn calculate_spread_chains(
        &self,
        initial: &[u8],
        final_flooded: &[u8],
    ) -> Vec<SpreadChain> {
        let mut chains = Vec::new();
        let open_doors: HashSet<u8> = self
            .door_states
            .iter()
            .filter(|d| d.is_open)
            .map(|d| d.bulkhead_id)
            .collect();

        for &start in initial {
            let mut visited = HashSet::new();
            let mut queue = vec![start];
            visited.insert(start);

            while let Some(current) = queue.pop() {
                if current > 0 && open_doors.contains(&(current - 1)) {
                    let left = current - 1;
                    if !visited.contains(&left) && final_flooded.contains(&left) {
                        visited.insert(left);
                        queue.push(left);
                    }
                }
                let right = current + 1;
                if right < self.hydrostatics.config().compartment_count
                    && open_doors.contains(&current)
                {
                    if !visited.contains(&right) && final_flooded.contains(&right) {
                        visited.insert(right);
                        queue.push(right);
                    }
                }
            }

            let spread_to: Vec<u8> = visited
                .iter()
                .filter(|&&c| !initial.contains(&c))
                .cloned()
                .collect();

            if !spread_to.is_empty() {
                chains.push(SpreadChain {
                    source_compartment: start,
                    spread_to_compartments: spread_to,
                    via_doors: open_doors
                        .iter()
                        .filter(|&&d| {
                            let left = d;
                            let right = d + 1;
                            visited.contains(&left) && visited.contains(&right)
                        })
                        .cloned()
                        .collect(),
                });
            }
        }

        chains
    }

    pub fn get_door_status_summary(&self) -> DoorStatusSummary {
        let total = self.door_states.len();
        let open_count = self.door_states.iter().filter(|d| d.is_open).count();
        let closed_count = total - open_count;

        DoorStatusSummary {
            total_doors: total as u8,
            open_doors: open_count as u8,
            closed_doors: closed_count as u8,
            open_ratio: if total > 0 {
                open_count as f64 / total as f64
            } else {
                0.0
            },
            watertight_integrity: if total > 0 {
                closed_count as f64 / total as f64
            } else {
                1.0
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VRSimulationResult {
    pub simulation_id: uuid::Uuid,
    pub ship_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub initial_damage: Vec<u8>,
    pub effective_flooded: Vec<u8>,
    pub damage_severity: f64,
    pub door_states: Vec<BulkheadDoorState>,
    pub spread_through_doors: bool,
    pub spread_chains: Vec<SpreadChain>,
    pub stability_result: StabilityResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadChain {
    pub source_compartment: u8,
    pub spread_to_compartments: Vec<u8>,
    pub via_doors: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorStatusSummary {
    pub total_doors: u8,
    pub open_doors: u8,
    pub closed_doors: u8,
    pub open_ratio: f64,
    pub watertight_integrity: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DamageParams;

    fn test_config() -> ShipConfig {
        ShipConfig {
            ship_id: "test_vr".to_string(),
            ship_name: "VR测试船".to_string(),
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

    fn test_simulator() -> VRCompartmentSimulator {
        let hydro = ShipHydrostatics::new(test_config(), DamageParams::default());
        VRCompartmentSimulator::new(hydro)
    }

    #[test]
    fn test_vr_simulator_new() {
        let sim = test_simulator();
        assert_eq!(sim.door_states().len(), 5);
        assert!(sim.door_states().iter().all(|d| !d.is_open));
    }

    #[test]
    fn test_set_door_state_valid() {
        let mut sim = test_simulator();
        assert!(sim.open_door(2).is_ok());
        assert!(sim.door_states()[2].is_open);
        assert!(sim.close_door(2).is_ok());
        assert!(!sim.door_states()[2].is_open);
    }

    #[test]
    fn test_set_door_state_invalid() {
        let mut sim = test_simulator();
        assert!(sim.open_door(10).is_err());
    }

    #[test]
    fn test_close_all_doors() {
        let mut sim = test_simulator();
        sim.open_all_doors();
        assert_eq!(sim.door_states().iter().filter(|d| d.is_open).count(), 5);
        sim.close_all_doors();
        assert_eq!(sim.door_states().iter().filter(|d| d.is_open).count(), 0);
    }

    #[test]
    fn test_simulate_closed_doors_no_spread() {
        let sim = test_simulator();
        let scenario = FloodingScenario {
            ship_id: "test_vr".to_string(),
            flooded_compartments: vec![2],
            damage_severity: 0.5,
        };
        let result = sim.simulate_with_door_states(&scenario);
        assert!(!result.spread_through_doors);
        assert_eq!(result.effective_flooded, vec![2]);
    }

    #[test]
    fn test_simulate_open_doors_spreads() {
        let mut sim = test_simulator();
        sim.open_door(2).unwrap();
        sim.open_door(3).unwrap();

        let scenario = FloodingScenario {
            ship_id: "test_vr".to_string(),
            flooded_compartments: vec![2],
            damage_severity: 0.5,
        };
        let result = sim.simulate_with_door_states(&scenario);
        assert!(result.spread_through_doors);
        assert!(result.effective_flooded.len() > 1);
        assert!(result.spread_chains.len() > 0);
    }

    #[test]
    fn test_door_status_summary() {
        let mut sim = test_simulator();
        let summary = sim.get_door_status_summary();
        assert_eq!(summary.total_doors, 5);
        assert_eq!(summary.open_doors, 0);
        assert_eq!(summary.closed_doors, 5);
        assert_eq!(summary.watertight_integrity, 1.0);

        sim.open_door(1).unwrap();
        sim.open_door(3).unwrap();
        let summary2 = sim.get_door_status_summary();
        assert_eq!(summary2.open_doors, 2);
        assert_eq!(summary2.closed_doors, 3);
        assert!((summary2.open_ratio - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_spread_chains_correct() {
        let mut sim = test_simulator();
        sim.open_door(1).unwrap();
        sim.open_door(2).unwrap();

        let scenario = FloodingScenario {
            ship_id: "test".to_string(),
            flooded_compartments: vec![1],
            damage_severity: 0.5,
        };
        let result = sim.simulate_with_door_states(&scenario);

        if result.spread_through_doors {
            assert!(!result.spread_chains.is_empty());
            let chain = &result.spread_chains[0];
            assert_eq!(chain.source_compartment, 1);
            assert!(chain.spread_to_compartments.len() >= 1);
        }
    }
}
