use crate::hydrostatics::ShipHydrostatics;
use crate::models::*;
use crate::ship_configs::{get_builtin_ship_configs, ship_config_extended_to_base};
use std::collections::HashMap;

pub struct EraComparator {
    damage_params: DamageParams,
}

impl EraComparator {
    pub fn new(damage_params: DamageParams) -> Self {
        Self { damage_params }
    }

    pub fn damage_params(&self) -> &DamageParams {
        &self.damage_params
    }

    pub fn compare_eras(&self, request: &EraComparisonRequest) -> Result<EraComparisonResult, String> {
        let all_configs = get_builtin_ship_configs();

        let ancient_ship = all_configs
            .iter()
            .find(|s| s.ship_id == request.ancient_ship_id)
            .cloned()
            .ok_or_else(|| format!("Ancient ship not found: {}", request.ancient_ship_id))?;

        let modern_ship = all_configs
            .iter()
            .find(|s| s.ship_id == request.modern_ship_id)
            .cloned()
            .ok_or_else(|| format!("Modern ship not found: {}", request.modern_ship_id))?;

        let key_metrics = self.generate_era_key_metrics(&ancient_ship, &modern_ship);

        let mut simulations = Vec::new();

        let test_scenarios = vec![
            ("单舱破损", vec![3u8], 0.5),
            ("相邻双舱破损", vec![3u8, 4u8], 0.6),
            ("三舱连续破损", vec![2u8, 3u8, 4u8], 0.7),
            ("机舱破损", vec![9u8], 0.8),
        ];

        for (scenario_name, compartments, severity) in test_scenarios {
            let ancient_result = self.run_scenario_for_comparison(
                &ancient_ship,
                &compartments,
                severity,
            );

            let modern_result = self.run_scenario_for_comparison(
                &modern_ship,
                &compartments,
                severity,
            );

            let winner = if ancient_result.is_safe && modern_result.is_safe {
                if ancient_result.metacentric_height > modern_result.metacentric_height {
                    "古船（GM更高）".to_string()
                } else {
                    "现代船（GM更高）".to_string()
                }
            } else if ancient_result.is_safe {
                "古船".to_string()
            } else if modern_result.is_safe {
                "现代船".to_string()
            } else {
                "均沉没".to_string()
            };

            let analysis = format!(
                "{}场景下，古船GM={:.3}m，储备浮力={:.1}%；现代船GM={:.3}m，储备浮力={:.1}%。{}",
                scenario_name,
                ancient_result.metacentric_height,
                ancient_result.reserve_buoyancy,
                modern_result.metacentric_height,
                modern_result.reserve_buoyancy,
                if ancient_result.is_safe && modern_result.is_safe {
                    "两者均保持安全，体现了水密隔舱技术的有效性。".to_string()
                } else if ancient_result.is_safe {
                    "古船设计在该场景下表现更优，证明古代水密隔舱技术的先进性。".to_string()
                } else if modern_result.is_safe {
                    "现代船凭借双层底、边舱等额外设计表现更优。".to_string()
                } else {
                    "该破损程度超出了两船的抗沉极限。".to_string()
                }
            );

            simulations.push(ComparativeSimulation {
                scenario: scenario_name.to_string(),
                ancient_result,
                modern_result,
                winner,
                analysis,
            });
        }

        let era_comparison = EraComparison {
            timeline: "唐代（7-10世纪）中国发明水密隔舱技术 → 宋代（10-13世纪）技术成熟，泉州宋船为代表 → 18世纪欧洲开始引入水密舱壁概念 → 1914年SOLAS公约确立现代分舱标准 → 20世纪后期双层底、边舱等技术完善".to_string(),
            technology_evolution: "水密隔舱技术从中国古代的经验性设计，发展为现代基于精确流体力学和稳性计算的科学化设计。核心思想一脉相承：将船体分隔为多个独立舱室，一舱或数舱进水不致沉没。现代技术增加了双层底、边舱、防撞舱壁等设计，计算方法更加精细。".to_string(),
            design_philosophy_difference: "古代设计注重实践经验和结构强度，隔舱布置兼顾抗沉性与使用功能；现代设计严格遵循SOLAS公约等国际规范，通过破损稳性计算确定分舱方案，追求安全性与经济性的平衡。".to_string(),
            regulatory_framework: "古代无统一规范，凭工匠经验和传统；现代有IMO/SOLAS、IMO/MARPOL等国际公约，各国船级社（如CCS、LR、ABS）制定详细规范。".to_string(),
            key_metrics_comparison: key_metrics,
        };

        Ok(EraComparisonResult {
            ancient_ship,
            modern_ship,
            era_comparison,
            simulation_results: simulations,
        })
    }

    fn generate_era_key_metrics(
        &self,
        ancient: &ShipConfigExtended,
        modern: &ShipConfigExtended,
    ) -> Vec<ComparisonMetric> {
        let mut metrics = Vec::new();

        let mut length_map = HashMap::new();
        length_map.insert(ancient.ship_id.clone(), ancient.length_overall);
        length_map.insert(modern.ship_id.clone(), modern.length_overall);
        metrics.push(ComparisonMetric {
            name: "总长".to_string(),
            ship_values: length_map,
            unit: "m".to_string(),
            description: "船舶总长".to_string(),
        });

        let mut disp_map = HashMap::new();
        disp_map.insert(ancient.ship_id.clone(), ancient.displacement);
        disp_map.insert(modern.ship_id.clone(), modern.displacement);
        metrics.push(ComparisonMetric {
            name: "排水量".to_string(),
            ship_values: disp_map,
            unit: "吨".to_string(),
            description: "满载排水量".to_string(),
        });

        let mut comp_map = HashMap::new();
        comp_map.insert(ancient.ship_id.clone(), ancient.compartment_count as f64);
        comp_map.insert(modern.ship_id.clone(), modern.compartment_count as f64);
        metrics.push(ComparisonMetric {
            name: "隔舱数量".to_string(),
            ship_values: comp_map,
            unit: "个".to_string(),
            description: "水密隔舱总数".to_string(),
        });

        let mut avg_len_map = HashMap::new();
        avg_len_map.insert(
            ancient.ship_id.clone(),
            ancient.length_overall / ancient.compartment_count as f64,
        );
        avg_len_map.insert(
            modern.ship_id.clone(),
            modern.length_overall / modern.compartment_count as f64,
        );
        metrics.push(ComparisonMetric {
            name: "平均舱长".to_string(),
            ship_values: avg_len_map,
            unit: "m".to_string(),
            description: "平均每个隔舱的长度，越短抗沉性越好".to_string(),
        });

        let mut eff_map = HashMap::new();
        eff_map.insert(ancient.ship_id.clone(), 1.0);
        let modern_efficiency = modern.comparison_with_ancient
            .as_ref()
            .map(|c| c.efficiency_gain)
            .unwrap_or(2.96);
        eff_map.insert(modern.ship_id.clone(), modern_efficiency);
        metrics.push(ComparisonMetric {
            name: "分舱效率系数".to_string(),
            ship_values: eff_map,
            unit: "相对值".to_string(),
            description: "单位排水量的抗沉能力，以古船为基准1.0，综合考虑SOLAS分舱指数R、双层底/边舱增益、隔舱优化".to_string(),
        });

        metrics
    }

    fn run_scenario_for_comparison(
        &self,
        ship: &ShipConfigExtended,
        compartments: &[u8],
        severity: f64,
    ) -> StabilityResultSummary {
        let config = ship_config_extended_to_base(ship);
        let scenario = FloodingScenario {
            ship_id: ship.ship_id.clone(),
            flooded_compartments: compartments.to_vec(),
            damage_severity: severity,
        };

        let hydrostatics = ShipHydrostatics::new(config, self.damage_params.clone());
        let result = hydrostatics.simulate_damage(&scenario);

        StabilityResultSummary {
            is_safe: result.is_safe,
            final_draft: result.final_draft,
            metacentric_height: result.metacentric_height,
            righting_arm_max: result.righting_arm_max,
            sinking_time_seconds: result.sinking_time_seconds,
            reserve_buoyancy: result.reserve_buoyancy,
        }
    }

    pub fn calculate_efficiency_ratio(
        &self,
        ancient_id: &str,
        modern_id: &str,
    ) -> Option<f64> {
        let all_configs = get_builtin_ship_configs();
        let ancient = all_configs.iter().find(|s| s.ship_id == ancient_id)?;
        let modern = all_configs.iter().find(|s| s.ship_id == modern_id)?;

        let ancient_config = ship_config_extended_to_base(ancient);
        let modern_config = ship_config_extended_to_base(modern);

        let ancient_hydro = ShipHydrostatics::new(ancient_config, self.damage_params.clone());
        let modern_hydro = ShipHydrostatics::new(modern_config, self.damage_params.clone());

        let ancient_max = ancient_hydro.calculate_max_floodable_compartments() as f64;
        let modern_max = modern_hydro.calculate_max_floodable_compartments() as f64;

        if ancient_max > 0.0 {
            Some(modern_max / ancient_max)
        } else {
            None
        }
    }

    pub fn get_era_timeline(&self) -> &'static str {
        "唐代（7-10世纪）中国发明水密隔舱技术 → 宋代（10-13世纪）技术成熟，泉州宋船为代表 → 18世纪欧洲开始引入水密舱壁概念 → 1914年SOLAS公约确立现代分舱标准 → 20世纪后期双层底、边舱等技术完善"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DamageParams;

    fn default_params() -> DamageParams {
        DamageParams::default()
    }

    fn test_comparator() -> EraComparator {
        EraComparator::new(default_params())
    }

    #[test]
    fn test_era_comparator_new() {
        let c = test_comparator();
        assert_eq!(c.damage_params().gravity, 9.81);
    }

    #[test]
    fn test_compare_eras_valid_ships() {
        let comparator = test_comparator();
        let request = EraComparisonRequest {
            ancient_ship_id: "quanzhou_song_001".to_string(),
            modern_ship_id: "modern_cargo_001".to_string(),
        };
        let result = comparator.compare_eras(&request);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.ancient_ship.ship_id, "quanzhou_song_001");
        assert_eq!(result.modern_ship.ship_id, "modern_cargo_001");
        assert_eq!(result.simulation_results.len(), 4);
        assert!(!result.era_comparison.timeline.is_empty());
    }

    #[test]
    fn test_compare_eras_invalid_ancient() {
        let comparator = test_comparator();
        let request = EraComparisonRequest {
            ancient_ship_id: "fake_ship".to_string(),
            modern_ship_id: "modern_cargo_001".to_string(),
        };
        let result = comparator.compare_eras(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_compare_eras_invalid_modern() {
        let comparator = test_comparator();
        let request = EraComparisonRequest {
            ancient_ship_id: "quanzhou_song_001".to_string(),
            modern_ship_id: "fake_ship".to_string(),
        };
        let result = comparator.compare_eras(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_era_key_metrics() {
        use crate::ship_configs::get_builtin_ship_configs;
        let comparator = test_comparator();
        let ships = get_builtin_ship_configs();
        let ancient = ships.iter().find(|s| s.ship_id == "quanzhou_song_001").unwrap();
        let modern = ships.iter().find(|s| s.ship_id == "modern_cargo_001").unwrap();

        let metrics = comparator.generate_era_key_metrics(ancient, modern);

        assert_eq!(metrics.len(), 5, "跨时代关键指标应该有5项");
        assert!(metrics.iter().any(|m| m.name == "分舱效率系数"));

        let efficiency = metrics.iter().find(|m| m.name == "分舱效率系数").unwrap();
        assert_eq!(efficiency.ship_values.get(&ancient.ship_id).copied(), Some(1.0),
                "古船分舱效率系数应为基准1.0");
        let modern_eff = efficiency.ship_values.get(&modern.ship_id).copied().unwrap_or(0.0);
        assert!(modern_eff > 2.5,
                "现代船分舱效率系数应大于2.5");
        assert!(modern_eff < 3.5,
                "现代船分舱效率系数应小于3.5");
    }

    #[test]
    fn test_run_scenario_for_comparison() {
        use crate::ship_configs::get_builtin_ship_configs;
        let comparator = test_comparator();
        let ships = get_builtin_ship_configs();
        let ship = ships.iter().find(|s| s.ship_id == "quanzhou_song_001").unwrap();

        let result = comparator.run_scenario_for_comparison(ship, &[3], 0.5);
        assert!(result.metacentric_height > 0.0);
        assert!(result.reserve_buoyancy > 0.0);
    }

    #[test]
    fn test_calculate_efficiency_ratio() {
        let comparator = test_comparator();
        let ratio = comparator.calculate_efficiency_ratio("quanzhou_song_001", "modern_cargo_001");
        assert!(ratio.is_some());
        let ratio = ratio.unwrap();
        assert!(ratio > 0.0);
    }

    #[test]
    fn test_get_era_timeline() {
        let comparator = test_comparator();
        let timeline = comparator.get_era_timeline();
        assert!(!timeline.is_empty());
        assert!(timeline.contains("唐代"));
        assert!(timeline.contains("SOLAS"));
    }

    #[test]
    fn test_simulation_winner_logic() {
        let comparator = test_comparator();
        let request = EraComparisonRequest {
            ancient_ship_id: "quanzhou_song_001".to_string(),
            modern_ship_id: "modern_cargo_001".to_string(),
        };
        let result = comparator.compare_eras(&request).unwrap();

        for sim in &result.simulation_results {
            assert!(!sim.winner.is_empty());
            assert!(
                sim.winner == "古船"
                    || sim.winner == "现代船"
                    || sim.winner == "均沉没"
                    || sim.winner.contains("GM更高"),
                "胜者字段应该是预期值之一，实际为: {}",
                sim.winner
            );
        }
    }
}
