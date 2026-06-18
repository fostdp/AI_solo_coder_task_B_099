use crate::hydrostatics::ShipHydrostatics;
use crate::models::*;
use crate::ship_configs::{get_builtin_ship_configs, ship_config_extended_to_base};
use std::collections::HashMap;

pub struct DesignComparator {
    damage_params: DamageParams,
}

impl DesignComparator {
    pub fn new(damage_params: DamageParams) -> Self {
        Self { damage_params }
    }

    pub fn damage_params(&self) -> &DamageParams {
        &self.damage_params
    }

    pub fn compare_ships(&self, ship_ids: &[String]) -> ShipComparisonResult {
        let all_configs = get_builtin_ship_configs();
        let ships: Vec<ShipConfigExtended> = all_configs
            .into_iter()
            .filter(|s| ship_ids.contains(&s.ship_id))
            .collect();

        let mut metrics = Vec::new();

        let mut length_map = HashMap::new();
        let mut displacement_map = HashMap::new();
        let mut compartment_map = HashMap::new();
        let mut avg_compartment_length_map = HashMap::new();
        let mut max_safe_flooded_map = HashMap::new();

        for ship in &ships {
            let config = ship_config_extended_to_base(ship);
            let hydrostatics = ShipHydrostatics::new(config, self.damage_params.clone());
            let max_flooded = hydrostatics.calculate_max_floodable_compartments();

            length_map.insert(ship.ship_id.clone(), ship.length_overall);
            displacement_map.insert(ship.ship_id.clone(), ship.displacement);
            compartment_map.insert(ship.ship_id.clone(), ship.compartment_count as f64);
            avg_compartment_length_map.insert(
                ship.ship_id.clone(),
                ship.length_overall / ship.compartment_count as f64,
            );
            max_safe_flooded_map.insert(ship.ship_id.clone(), max_flooded as f64);
        }

        metrics.push(ComparisonMetric {
            name: "总长".to_string(),
            ship_values: length_map,
            unit: "m".to_string(),
            description: "船舶总长，越长通常抗沉性越好但机动性下降".to_string(),
        });

        metrics.push(ComparisonMetric {
            name: "排水量".to_string(),
            ship_values: displacement_map,
            unit: "吨".to_string(),
            description: "满载排水量，代表船舶大小和载货能力".to_string(),
        });

        metrics.push(ComparisonMetric {
            name: "隔舱数量".to_string(),
            ship_values: compartment_map,
            unit: "个".to_string(),
            description: "水密隔舱数量，越多抗沉性越好但建造成本越高".to_string(),
        });

        metrics.push(ComparisonMetric {
            name: "平均舱长".to_string(),
            ship_values: avg_compartment_length_map,
            unit: "m".to_string(),
            description: "平均隔舱长度，越短抗沉性越好".to_string(),
        });

        metrics.push(ComparisonMetric {
            name: "最大可进水舱数".to_string(),
            ship_values: max_safe_flooded_map,
            unit: "舱".to_string(),
            description: "在严重度0.5破损情况下仍能保持安全的最大进水舱数".to_string(),
        });

        let conclusion = self.generate_comparison_conclusion(&ships, &metrics);

        ShipComparisonResult {
            ships,
            comparison_metrics: metrics,
            conclusion,
        }
    }

    pub fn generate_comparison_conclusion(
        &self,
        ships: &[ShipConfigExtended],
        metrics: &[ComparisonMetric],
    ) -> String {
        if ships.len() < 2 {
            return "请选择至少两艘船舶进行对比".to_string();
        }

        let max_flooded_metric = metrics.iter().find(|m| m.name == "最大可进水舱数");
        let compartment_metric = metrics.iter().find(|m| m.name == "隔舱数量");

        let mut conclusion = String::new();

        if let (Some(mf), Some(cc)) = (max_flooded_metric, compartment_metric) {
            let best_ship = mf
                .ship_values
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(id, _)| ships.iter().find(|s| &s.ship_id == id).map(|s| s.ship_name.clone()))
                .flatten();

            if let Some(ship_name) = best_ship {
                conclusion.push_str(&format!(
                    "综合抗沉性最优的是「{}」。",
                    ship_name
                ));
            }

            let ancient_ships: Vec<_> = ships.iter().filter(|s| s.ship_type != "modern").collect();
            let modern_ships: Vec<_> = ships.iter().filter(|s| s.ship_type == "modern").collect();

            if !ancient_ships.is_empty() && !modern_ships.is_empty() {
                conclusion.push_str(
                    " 古代水密隔舱设计与现代SOLAS分舱标准一脉相承，核心思想都是通过横向水密舱壁将船体分隔为独立舱室。"
                );
            }

            if ancient_ships.len() >= 2 {
                conclusion.push_str(
                    " 中国古代四大船型（福船、沙船、广船、泉州宋船）各具特色：福船高大如楼适合深海作战，沙船平底浅吃水适合漕运，广船修长尖底适合远洋贸易。"
                );
            }
        }

        if conclusion.is_empty() {
            conclusion = "对比分析完成，请查看详细指标。".to_string();
        }

        conclusion
    }

    pub fn find_best_ship(&self, ship_ids: &[String]) -> Option<ShipConfigExtended> {
        let result = self.compare_ships(ship_ids);
        if result.ships.is_empty() {
            return None;
        }

        result.comparison_metrics
            .iter()
            .find(|m| m.name == "最大可进水舱数")
            .and_then(|metric| {
                metric.ship_values
                    .iter()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(id, _)| id.clone())
            })
            .and_then(|best_id| {
                result.ships.into_iter().find(|s| s.ship_id == best_id)
            })
    }

    pub fn compare_by_metric(
        &self,
        ship_ids: &[String],
        metric_name: &str,
    ) -> Option<Vec<(String, f64)>> {
        let result = self.compare_ships(ship_ids);
        result.comparison_metrics
            .iter()
            .find(|m| m.name == metric_name)
            .map(|metric| {
                let mut entries: Vec<(String, f64)> = metric
                    .ship_values
                    .iter()
                    .map(|(k, v)| (k.clone(), *v))
                    .collect();
                entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                entries
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DamageParams;

    fn default_params() -> DamageParams {
        DamageParams::default()
    }

    fn test_comparator() -> DesignComparator {
        DesignComparator::new(default_params())
    }

    #[test]
    fn test_design_comparator_new() {
        let c = test_comparator();
        assert_eq!(c.damage_params().gravity, 9.81);
    }

    #[test]
    fn test_compare_ships_two_ships_normal() {
        let comparator = test_comparator();
        let ship_ids = vec![
            "quanzhou_song_001".to_string(),
            "fu_ship_001".to_string(),
        ];
        let result = comparator.compare_ships(&ship_ids);

        assert_eq!(result.ships.len(), 2, "应该对比2艘船");
        assert!(!result.comparison_metrics.is_empty(), "应该有对比指标");
        assert!(!result.conclusion.is_empty(), "应该有结论");
        assert!(result.conclusion.contains("最优"));
    }

    #[test]
    fn test_compare_ships_three_ancient_ships() {
        let comparator = test_comparator();
        let ship_ids = vec![
            "quanzhou_song_001".to_string(),
            "fu_ship_001".to_string(),
            "sha_ship_001".to_string(),
            "guang_ship_001".to_string(),
        ];
        let result = comparator.compare_ships(&ship_ids);

        assert_eq!(result.ships.len(), 4);
        assert!(result.comparison_metrics.len() >= 5);

        let metric_names: Vec<_> = result.comparison_metrics.iter().map(|m| m.name.as_str()).collect();
        assert!(metric_names.contains(&"总长"));
        assert!(metric_names.contains(&"排水量"));
        assert!(metric_names.contains(&"隔舱数量"));
        assert!(metric_names.contains(&"平均舱长"));
        assert!(metric_names.contains(&"最大可进水舱数"));
    }

    #[test]
    fn test_compare_ships_ancient_vs_modern() {
        let comparator = test_comparator();
        let ship_ids = vec![
            "quanzhou_song_001".to_string(),
            "modern_cargo_001".to_string(),
        ];
        let result = comparator.compare_ships(&ship_ids);

        assert!(result.conclusion.contains("SOLAS") || result.conclusion.contains("一脉相承"),
                "跨时代对比应该提到SOLAS或一脉相承");
    }

    #[test]
    fn test_compare_ships_single_ship() {
        let comparator = test_comparator();
        let ship_ids = vec!["quanzhou_song_001".to_string()];
        let result = comparator.compare_ships(&ship_ids);

        assert_eq!(result.ships.len(), 1, "单船对比也应返回1艘船的数据");
        assert_eq!(result.conclusion, "请选择至少两艘船舶进行对比",
                "单船对比应该提示需要至少两艘船");
    }

    #[test]
    fn test_compare_ships_empty_list() {
        let comparator = test_comparator();
        let ship_ids: Vec<String> = vec![];
        let result = comparator.compare_ships(&ship_ids);

        assert!(result.ships.is_empty(), "空列表应返回空船舶列表");
        assert!(!result.comparison_metrics.is_empty(), "即使空列表指标列表也应存在");
    }

    #[test]
    fn test_compare_ships_nonexistent() {
        let comparator = test_comparator();
        let ship_ids = vec![
            "fake_ship_1".to_string(),
            "fake_ship_2".to_string(),
        ];
        let result = comparator.compare_ships(&ship_ids);

        assert!(result.ships.is_empty(), "不存在的船应该返回空列表");
    }

    #[test]
    fn test_max_flooded_metric_sane() {
        let comparator = test_comparator();
        let ship_ids = vec![
            "quanzhou_song_001".to_string(),
            "fu_ship_001".to_string(),
        ];
        let result = comparator.compare_ships(&ship_ids);

        let max_flooded = result.comparison_metrics.iter()
            .find(|m| m.name == "最大可进水舱数")
            .expect("应该有最大可进水舱数指标");

        for (_, &value) in &max_flooded.ship_values {
            assert!(value >= 0.0, "最大可进水舱数不应为负");
            assert!(value <= 20.0, "最大可进水舱数不应超过20");
        }
    }

    #[test]
    fn test_find_best_ship() {
        let comparator = test_comparator();
        let ship_ids = vec![
            "quanzhou_song_001".to_string(),
            "fu_ship_001".to_string(),
            "sha_ship_001".to_string(),
        ];
        let best = comparator.find_best_ship(&ship_ids);
        assert!(best.is_some());
        let best = best.unwrap();
        assert!(["quanzhou_song_001", "fu_ship_001", "sha_ship_001"]
            .contains(&best.ship_id.as_str()));
    }

    #[test]
    fn test_find_best_ship_empty() {
        let comparator = test_comparator();
        let best = comparator.find_best_ship(&[]);
        assert!(best.is_none());
    }

    #[test]
    fn test_compare_by_metric() {
        let comparator = test_comparator();
        let ship_ids = vec![
            "quanzhou_song_001".to_string(),
            "fu_ship_001".to_string(),
        ];
        let result = comparator.compare_by_metric(&ship_ids, "总长");
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].1 >= result[1].1, "应该按降序排列");
    }

    #[test]
    fn test_compare_by_metric_invalid() {
        let comparator = test_comparator();
        let ship_ids = vec!["quanzhou_song_001".to_string()];
        let result = comparator.compare_by_metric(&ship_ids, "不存在的指标");
        assert!(result.is_none());
    }
}
