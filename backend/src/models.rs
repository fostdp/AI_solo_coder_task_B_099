use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipConfig {
    pub ship_id: String,
    pub ship_name: String,
    pub length_overall: f64,
    pub beam: f64,
    pub depth: f64,
    pub design_draft: f64,
    pub displacement: f64,
    pub compartment_count: u8,
    pub compartment_names: Vec<String>,
    pub compartment_lengths: Vec<f64>,
    pub compartment_volumes: Vec<f64>,
    pub watertight_bulkheads: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorData {
    pub ship_id: String,
    pub timestamp: DateTime<Utc>,
    pub compartment_id: u8,
    pub water_level: f64,
    pub max_water_level: f64,
    pub is_flooded: bool,
    pub draft: f64,
    pub heel_angle: f64,
    pub trim_angle: f64,
    pub damage_location: String,
    pub damage_severity: f64,
    pub metacentric_height: f64,
    pub righting_arm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompartmentState {
    pub compartment_id: u8,
    pub water_level: f64,
    pub volume_flooded: f64,
    pub is_flooded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloodingScenario {
    pub ship_id: String,
    pub flooded_compartments: Vec<u8>,
    pub damage_severity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilityResult {
    pub simulation_id: Uuid,
    pub ship_id: String,
    pub timestamp: DateTime<Utc>,
    pub flooded_compartments: Vec<u8>,
    pub final_draft: f64,
    pub final_heel_angle: f64,
    pub final_trim_angle: f64,
    pub metacentric_height: f64,
    pub righting_arm_max: f64,
    pub range_of_stability: f64,
    pub is_safe: bool,
    pub sinking_time_seconds: f64,
    pub reserve_buoyancy: f64,
    pub stability_curve: Vec<StabilityPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilityPoint {
    pub heel_angle: f64,
    pub righting_arm: f64,
    pub righting_moment: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlarmType {
    StabilityLoss,
    FloodingSpread,
    DraftExceeded,
    HeelExcessive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlarmLevel {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmEvent {
    pub alarm_id: Uuid,
    pub ship_id: String,
    pub timestamp: DateTime<Utc>,
    pub alarm_type: AlarmType,
    pub alarm_level: AlarmLevel,
    pub description: String,
    pub flooded_compartments: Vec<u8>,
    pub metacentric_height: f64,
    pub heel_angle: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRequest {
    pub ship_id: String,
    pub min_compartments: u8,
    pub max_compartments: u8,
    pub population_size: usize,
    pub generations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub optimization_id: Uuid,
    pub ship_id: String,
    pub timestamp: DateTime<Utc>,
    pub compartment_count: u8,
    pub fitness_score: f64,
    pub max_flooded_compartments: u8,
    pub survival_probability: f64,
    pub configuration: Vec<f64>,
    pub best_configurations: Vec<OptimizedConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedConfig {
    pub compartment_count: u8,
    pub bulkhead_positions: Vec<f64>,
    pub fitness: f64,
    pub survival_probability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub message_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageParams {
    pub gravity: f64,
    pub sea_water_density: f64,
    pub permeability: f64,
    pub min_metacentric_height: f64,
    pub max_safe_heel_angle: f64,
    pub min_reserve_buoyancy: f64,
    pub draft_depth_ratio_threshold: f64,
    pub flooding_spread_count: usize,
    pub block_coefficient_base: f64,
    pub block_coefficient_draft_factor: f64,
    pub waterplane_coefficient_base: f64,
    pub waterplane_coefficient_draft_factor: f64,
    pub hull_form_factor: f64,
    pub damage_orifice_area_coefficient: f64,
    pub sinking_time_threshold_seconds: f64,
    pub max_safe_draft_depth_ratio: f64,
}

impl Default for DamageParams {
    fn default() -> Self {
        Self {
            gravity: 9.81,
            sea_water_density: 1025.0,
            permeability: 0.7,
            min_metacentric_height: 0.15,
            max_safe_heel_angle: 15.0,
            min_reserve_buoyancy: 10.0,
            draft_depth_ratio_threshold: 0.9,
            flooding_spread_count: 3,
            block_coefficient_base: 0.68,
            block_coefficient_draft_factor: 0.08,
            waterplane_coefficient_base: 0.75,
            waterplane_coefficient_draft_factor: 0.05,
            hull_form_factor: 0.7,
            damage_orifice_area_coefficient: 0.5,
            sinking_time_threshold_seconds: 3600.0,
            max_safe_draft_depth_ratio: 0.95,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipCharacteristics {
    pub hull_form: String,
    pub primary_use: String,
    pub notable_feature: String,
    pub max_safe_flooded: u8,
    #[serde(default)]
    pub double_bottom: bool,
    #[serde(default)]
    pub double_side: bool,
    #[serde(default)]
    pub solas_compliant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipComparisonInfo {
    pub technology_origin: String,
    pub similarity: String,
    pub difference: String,
    pub efficiency_gain: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipConfigExtended {
    pub ship_id: String,
    pub ship_name: String,
    #[serde(rename = "type")]
    pub ship_type: String,
    pub dynasty: String,
    pub historical_description: String,
    pub length_overall: f64,
    pub beam: f64,
    pub depth: f64,
    pub design_draft: f64,
    pub displacement: f64,
    pub compartment_count: u8,
    pub compartment_names: Vec<String>,
    pub compartment_lengths: Vec<f64>,
    pub compartment_volumes: Vec<f64>,
    pub watertight_bulkheads: Vec<f64>,
    pub characteristics: ShipCharacteristics,
    #[serde(default)]
    pub comparison_with_ancient: Option<ShipComparisonInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipComparisonRequest {
    pub ship_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipComparisonResult {
    pub ships: Vec<ShipConfigExtended>,
    pub comparison_metrics: Vec<ComparisonMetric>,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonMetric {
    pub name: String,
    pub ship_values: std::collections::HashMap<String, f64>,
    pub unit: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EraComparisonRequest {
    pub ancient_ship_id: String,
    pub modern_ship_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EraComparisonResult {
    pub ancient_ship: ShipConfigExtended,
    pub modern_ship: ShipConfigExtended,
    pub era_comparison: EraComparison,
    pub simulation_results: Vec<ComparativeSimulation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EraComparison {
    pub timeline: String,
    pub technology_evolution: String,
    pub design_philosophy_difference: String,
    pub regulatory_framework: String,
    pub key_metrics_comparison: Vec<ComparisonMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparativeSimulation {
    pub scenario: String,
    pub ancient_result: StabilityResultSummary,
    pub modern_result: StabilityResultSummary,
    pub winner: String,
    pub analysis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilityResultSummary {
    pub is_safe: bool,
    pub final_draft: f64,
    pub metacentric_height: f64,
    pub righting_arm_max: f64,
    pub sinking_time_seconds: f64,
    pub reserve_buoyancy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PirateAttackRequest {
    pub ship_id: String,
    pub attack_points: Vec<AttackPoint>,
    pub simulation_duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackPoint {
    pub compartment_id: u8,
    pub damage_severity: f64,
    pub delay_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PirateAttackResult {
    pub simulation_id: Uuid,
    pub ship_id: String,
    pub timestamp: DateTime<Utc>,
    pub attack_points: Vec<AttackPoint>,
    pub timeline_events: Vec<TimelineEvent>,
    pub final_state: StabilityResult,
    pub survival_probability: f64,
    pub critical_moments: Vec<CriticalMoment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub time_seconds: f64,
    pub event_type: String,
    pub description: String,
    pub affected_compartments: Vec<u8>,
    pub gm_value: f64,
    pub is_safe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalMoment {
    pub time_seconds: f64,
    pub description: String,
    pub gm_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkheadDoorState {
    pub bulkhead_id: u8,
    pub is_open: bool,
    pub last_changed: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorControlRequest {
    pub ship_id: String,
    pub bulkhead_id: u8,
    pub open: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveSimulationRequest {
    pub ship_id: String,
    pub flooded_compartments: Vec<u8>,
    pub damage_severity: f64,
    pub door_states: Vec<BulkheadDoorState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipListEntry {
    pub ship_id: String,
    pub ship_name: String,
    pub ship_type: String,
    pub dynasty: String,
    pub compartment_count: u8,
    pub length_overall: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllShipsResponse {
    pub ships: Vec<ShipListEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_damage_params_default() {
        let params = DamageParams::default();
        assert_eq!(params.gravity, 9.81);
        assert_eq!(params.sea_water_density, 1025.0);
        assert_eq!(params.permeability, 0.7);
        assert!(params.min_metacentric_height > 0.0);
        assert!(params.max_safe_heel_angle > 0.0);
    }

    #[test]
    fn test_damage_params_clone() {
        let p1 = DamageParams::default();
        let p2 = p1.clone();
        assert_eq!(p1.gravity, p2.gravity);
        assert_eq!(p1.permeability, p2.permeability);
    }

    #[test]
    fn test_flooding_scenario_serialization() {
        let scenario = FloodingScenario {
            ship_id: "test_001".to_string(),
            flooded_compartments: vec![1, 2, 3],
            damage_severity: 0.5,
        };

        let json = serde_json::to_string(&scenario).unwrap();
        let deserialized: FloodingScenario = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.ship_id, "test_001");
        assert_eq!(deserialized.flooded_compartments, vec![1, 2, 3]);
        assert_eq!(deserialized.damage_severity, 0.5);
    }

    #[test]
    fn test_attack_point_serialization() {
        let ap = AttackPoint {
            compartment_id: 5,
            damage_severity: 0.8,
            delay_seconds: 120.0,
        };

        let json = serde_json::to_string(&ap).unwrap();
        let result: AttackPoint = serde_json::from_str(&json).unwrap();

        assert_eq!(result.compartment_id, 5);
        assert_eq!(result.damage_severity, 0.8);
        assert_eq!(result.delay_seconds, 120.0);
    }

    #[test]
    fn test_pirate_attack_request_serialization() {
        let request = PirateAttackRequest {
            ship_id: "test_ship".to_string(),
            attack_points: vec![
                AttackPoint {
                    compartment_id: 2,
                    damage_severity: 0.6,
                    delay_seconds: 0.0,
                },
                AttackPoint {
                    compartment_id: 5,
                    damage_severity: 0.7,
                    delay_seconds: 100.0,
                },
            ],
            simulation_duration_seconds: 600.0,
        };

        let json = serde_json::to_string(&request).unwrap();
        let result: PirateAttackRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(result.ship_id, "test_ship");
        assert_eq!(result.attack_points.len(), 2);
        assert_eq!(result.simulation_duration_seconds, 600.0);
    }

    #[test]
    fn test_bulkhead_door_state_serialization() {
        let state = BulkheadDoorState {
            bulkhead_id: 3,
            is_open: true,
            last_changed: Utc::now(),
        };

        let json = serde_json::to_string(&state).unwrap();
        let result: BulkheadDoorState = serde_json::from_str(&json).unwrap();

        assert_eq!(result.bulkhead_id, 3);
        assert_eq!(result.is_open, true);
    }

    #[test]
    fn test_interactive_simulation_request_serialization() {
        let request = InteractiveSimulationRequest {
            ship_id: "test".to_string(),
            flooded_compartments: vec![2],
            damage_severity: 0.5,
            door_states: vec![
                BulkheadDoorState {
                    bulkhead_id: 1,
                    is_open: false,
                    last_changed: Utc::now(),
                },
                BulkheadDoorState {
                    bulkhead_id: 2,
                    is_open: true,
                    last_changed: Utc::now(),
                },
            ],
        };

        let json = serde_json::to_string(&request).unwrap();
        let result: InteractiveSimulationRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(result.ship_id, "test");
        assert_eq!(result.flooded_compartments, vec![2]);
        assert_eq!(result.door_states.len(), 2);
        assert_eq!(result.door_states[1].is_open, true);
    }

    #[test]
    fn test_comparison_metric_serialization() {
        use std::collections::HashMap;

        let mut values = HashMap::new();
        values.insert("ship_a".to_string(), 34.0);
        values.insert("ship_b".to_string(), 48.0);

        let metric = ComparisonMetric {
            name: "总长".to_string(),
            ship_values: values,
            unit: "m".to_string(),
            description: "船舶总长".to_string(),
        };

        let json = serde_json::to_string(&metric).unwrap();
        let result: ComparisonMetric = serde_json::from_str(&json).unwrap();

        assert_eq!(result.name, "总长");
        assert_eq!(result.unit, "m");
        assert_eq!(result.ship_values.len(), 2);
    }

    #[test]
    fn test_ship_comparison_request_empty() {
        let request = ShipComparisonRequest {
            ship_ids: vec![],
        };
        let json = serde_json::to_string(&request).unwrap();
        let result: ShipComparisonRequest = serde_json::from_str(&json).unwrap();
        assert!(result.ship_ids.is_empty());
    }

    #[test]
    fn test_era_comparison_request() {
        let req = EraComparisonRequest {
            ancient_ship_id: "quanzhou_song".to_string(),
            modern_ship_id: "modern_cargo".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let result: EraComparisonRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(result.ancient_ship_id, "quanzhou_song");
        assert_eq!(result.modern_ship_id, "modern_cargo");
    }

    #[test]
    fn test_door_control_request() {
        let req = DoorControlRequest {
            ship_id: "test".to_string(),
            bulkhead_id: 5,
            open: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        let result: DoorControlRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(result.bulkhead_id, 5);
        assert_eq!(result.open, true);
    }

    #[test]
    fn test_timeline_event_types() {
        let start = TimelineEvent {
            time_seconds: 0.0,
            event_type: "START".to_string(),
            description: "开始".to_string(),
            affected_compartments: vec![],
            gm_value: 0.5,
            is_safe: true,
        };
        assert!(start.is_safe);
        assert_eq!(start.event_type, "START");

        let unsafe_ev = TimelineEvent {
            time_seconds: 100.0,
            event_type: "UNSAFE".to_string(),
            description: "危险".to_string(),
            affected_compartments: vec![1, 2, 3],
            gm_value: 0.1,
            is_safe: false,
        };
        assert!(!unsafe_ev.is_safe);
        assert_eq!(unsafe_ev.affected_compartments.len(), 3);
    }

    #[test]
    fn test_stability_result_summary_default_safe() {
        let summary = StabilityResultSummary {
            is_safe: true,
            final_draft: 2.8,
            metacentric_height: 0.5,
            righting_arm_max: 0.3,
            sinking_time_seconds: 3600.0,
            reserve_buoyancy: 30.0,
        };
        assert!(summary.is_safe);
        assert!(summary.reserve_buoyancy > 0.0);
        assert!(summary.metacentric_height > 0.0);
    }

    #[test]
    fn test_critical_moment_fields() {
        let cm = CriticalMoment {
            time_seconds: 250.0,
            description: "GM降至安全阈值以下".to_string(),
            gm_value: 0.14,
        };
        assert_eq!(cm.time_seconds, 250.0);
        assert!(cm.description.contains("GM"));
        assert!(cm.gm_value < 0.15);
    }
}

