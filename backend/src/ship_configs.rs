use crate::models::*;

pub fn get_builtin_ship_configs() -> Vec<ShipConfigExtended> {
    vec![
        load_ship_config_from_json(include_str!("../config/ship_config.json"), "quanzhou", "宋代"),
        load_ship_config_from_json(include_str!("../config/fu_ship_config.json"), "fu", "明代"),
        load_ship_config_from_json(include_str!("../config/sha_ship_config.json"), "sha", "清代"),
        load_ship_config_from_json(include_str!("../config/guang_ship_config.json"), "guang", "明代"),
        load_ship_config_from_json(include_str!("../config/modern_ship_config.json"), "modern", "现代"),
    ]
}

fn load_ship_config_from_json(json_str: &str, ship_type: &str, dynasty: &str) -> ShipConfigExtended {
    let base: serde_json::Value = serde_json::from_str(json_str).unwrap();
    let characteristics = base.get("characteristics").and_then(|c| {
        serde_json::from_value::<ShipCharacteristics>(c.clone()).ok()
    }).unwrap_or_default();

    let comparison = base.get("comparison_with_ancient").and_then(|c| {
        serde_json::from_value::<ShipComparisonInfo>(c.clone()).ok()
    });

    ShipConfigExtended {
        ship_id: base.get("ship_id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
        ship_name: base.get("ship_name").and_then(|v| v.as_str()).unwrap_or("未知船舶").to_string(),
        ship_type: base.get("ship_type").and_then(|v| v.as_str()).unwrap_or(ship_type).to_string(),
        dynasty: base.get("dynasty").and_then(|v| v.as_str()).unwrap_or(dynasty).to_string(),
        historical_description: base.get("historical_description")
            .and_then(|v| v.as_str())
            .unwrap_or("中国古代水密隔舱海船").to_string(),
        length_overall: base.get("length_overall").and_then(|v| v.as_f64()).unwrap_or(34.0),
        beam: base.get("beam").and_then(|v| v.as_f64()).unwrap_or(11.0),
        depth: base.get("depth").and_then(|v| v.as_f64()).unwrap_or(4.5),
        design_draft: base.get("design_draft").and_then(|v| v.as_f64()).unwrap_or(2.8),
        displacement: base.get("displacement").and_then(|v| v.as_f64()).unwrap_or(400.0),
        compartment_count: base.get("compartment_count").and_then(|v| v.as_u64()).unwrap_or(13) as u8,
        compartment_names: base.get("compartment_names")
            .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
            .unwrap_or_default(),
        compartment_lengths: base.get("compartment_lengths")
            .and_then(|v| serde_json::from_value::<Vec<f64>>(v.clone()).ok())
            .unwrap_or_default(),
        compartment_volumes: base.get("compartment_volumes")
            .and_then(|v| serde_json::from_value::<Vec<f64>>(v.clone()).ok())
            .unwrap_or_default(),
        watertight_bulkheads: base.get("watertight_bulkheads")
            .and_then(|v| serde_json::from_value::<Vec<f64>>(v.clone()).ok())
            .unwrap_or_default(),
        characteristics,
        comparison_with_ancient: comparison,
    }
}

pub fn get_all_ships_list() -> AllShipsResponse {
    let ships = get_builtin_ship_configs()
        .iter()
        .map(|s| ShipListEntry {
            ship_id: s.ship_id.clone(),
            ship_name: s.ship_name.clone(),
            ship_type: s.ship_type.clone(),
            dynasty: s.dynasty.clone(),
            compartment_count: s.compartment_count,
            length_overall: s.length_overall,
        })
        .collect();
    AllShipsResponse { ships }
}

pub fn get_ship_config_extended(ship_id: &str) -> Option<ShipConfigExtended> {
    get_builtin_ship_configs()
        .into_iter()
        .find(|s| s.ship_id == ship_id)
}

pub fn ship_config_extended_to_base(extended: &ShipConfigExtended) -> ShipConfig {
    ShipConfig {
        ship_id: extended.ship_id.clone(),
        ship_name: extended.ship_name.clone(),
        length_overall: extended.length_overall,
        beam: extended.beam,
        depth: extended.depth,
        design_draft: extended.design_draft,
        displacement: extended.displacement,
        compartment_count: extended.compartment_count,
        compartment_names: extended.compartment_names.clone(),
        compartment_lengths: extended.compartment_lengths.clone(),
        compartment_volumes: extended.compartment_volumes.clone(),
        watertight_bulkheads: extended.watertight_bulkheads.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_builtin_ship_configs_count() {
        let ships = get_builtin_ship_configs();
        assert_eq!(ships.len(), 5, "应该有5艘内置船舶配置");
    }

    #[test]
    fn test_get_builtin_ship_configs_ids_unique() {
        let ships = get_builtin_ship_configs();
        let ids: std::collections::HashSet<_> = ships.iter().map(|s| s.ship_id.clone()).collect();
        assert_eq!(ids.len(), ships.len(), "所有船舶ID应该唯一");
    }

    #[test]
    fn test_get_builtin_ship_configs_basic_fields() {
        let ships = get_builtin_ship_configs();
        for ship in &ships {
            assert!(!ship.ship_id.is_empty(), "船舶ID不应为空");
            assert!(!ship.ship_name.is_empty(), "船舶名称不应为空");
            assert!(ship.length_overall > 0.0, "总长应大于0: {}", ship.ship_id);
            assert!(ship.displacement > 0.0, "排水量应大于0: {}", ship.ship_id);
            assert!(ship.compartment_count > 0, "隔舱数应大于0: {}", ship.ship_id);
        }
    }

    #[test]
    fn test_get_all_ships_list() {
        let response = get_all_ships_list();
        assert_eq!(response.ships.len(), 5, "列表应返回5艘船");
        for entry in &response.ships {
            assert!(!entry.ship_id.is_empty());
            assert!(!entry.ship_name.is_empty());
            assert!(entry.compartment_count > 0);
            assert!(entry.length_overall > 0.0);
        }
    }

    #[test]
    fn test_get_ship_config_extended_valid() {
        let config = get_ship_config_extended("quanzhou_song_001");
        assert!(config.is_some(), "应该能找到泉州宋船");
        let config = config.unwrap();
        assert_eq!(config.ship_id, "quanzhou_song_001");
        assert!(config.compartment_count >= 10);
    }

    #[test]
    fn test_get_ship_config_extended_invalid() {
        let config = get_ship_config_extended("nonexistent_ship_id");
        assert!(config.is_none(), "不存在的船应该返回None");
    }

    #[test]
    fn test_load_ship_config_from_json_structure() {
        let ships = get_builtin_ship_configs();
        for ship in &ships {
            assert!(!ship.compartment_names.is_empty(),
                    "船舶{}应该有舱室名称", ship.ship_id);
            assert_eq!(ship.compartment_names.len() as u8, ship.compartment_count,
                    "舱室名称数量应等于隔舱数: {}", ship.ship_id);
        }
    }

    #[test]
    fn test_ship_characteristics_present() {
        let ships = get_builtin_ship_configs();
        for ship in &ships {
            assert!(!ship.characteristics.hull_form.is_empty());
            assert!(!ship.characteristics.primary_use.is_empty());
            assert!(ship.characteristics.max_safe_flooded > 0);
        }
    }

    #[test]
    fn test_modern_ship_has_advanced_features() {
        let modern = get_ship_config_extended("modern_cargo_001").unwrap();
        assert!(modern.characteristics.solas_compliant, "现代船应符合SOLAS");
        assert!(modern.characteristics.double_bottom || modern.characteristics.double_side,
                "现代船应有双层底或边舱");
        assert!(modern.comparison_with_ancient.is_some(),
                "现代船应有与古船的对比信息");
    }

    #[test]
    fn test_ship_config_extended_to_base() {
        let extended = get_ship_config_extended("quanzhou_song_001").unwrap();
        let base = ship_config_extended_to_base(&extended);
        assert_eq!(base.ship_id, extended.ship_id);
        assert_eq!(base.length_overall, extended.length_overall);
        assert_eq!(base.compartment_count, extended.compartment_count);
    }
}
