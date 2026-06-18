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

