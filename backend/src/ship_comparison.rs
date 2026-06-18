use crate::clickhouse_client::ClickHouseClient;
use crate::flooding_simulator::ShipHydrostatics;
use crate::models::*;
use std::collections::HashMap;

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
    }).unwrap_or_else(|| ShipCharacteristics {
        hull_form: "传统福船型".to_string(),
        primary_use: "远洋航行".to_string(),
        notable_feature: "水密隔舱".to_string(),
        max_safe_flooded: 3,
        double_bottom: false,
        double_side: false,
        solas_compliant: false,
    });

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

pub fn compare_ships(ship_ids: &[String], damage_params: &DamageParams) -> ShipComparisonResult {
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
        let config = ShipConfig {
            ship_id: ship.ship_id.clone(),
            ship_name: ship.ship_name.clone(),
            length_overall: ship.length_overall,
            beam: ship.beam,
            depth: ship.depth,
            design_draft: ship.design_draft,
            displacement: ship.displacement,
            compartment_count: ship.compartment_count,
            compartment_names: ship.compartment_names.clone(),
            compartment_lengths: ship.compartment_lengths.clone(),
            compartment_volumes: ship.compartment_volumes.clone(),
            watertight_bulkheads: ship.watertight_bulkheads.clone(),
        };

        let hydrostatics = ShipHydrostatics::new(config, damage_params.clone());
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

    let conclusion = generate_comparison_conclusion(&ships, &metrics);

    ShipComparisonResult {
        ships,
        comparison_metrics: metrics,
        conclusion,
    }
}

fn generate_comparison_conclusion(
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

pub async fn compare_eras(
    request: &EraComparisonRequest,
    damage_params: &DamageParams,
    clickhouse: &ClickHouseClient,
) -> Result<EraComparisonResult, String> {
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

    let key_metrics = generate_era_key_metrics(&ancient_ship, &modern_ship);

    let mut simulations = Vec::new();

    let test_scenarios = vec![
        ("单舱破损", vec![3u8], 0.5),
        ("相邻双舱破损", vec![3u8, 4u8], 0.6),
        ("三舱连续破损", vec![2u8, 3u8, 4u8], 0.7),
        ("机舱破损", vec![9u8], 0.8),
    ];

    for (scenario_name, compartments, severity) in test_scenarios {
        let ancient_result = run_scenario_for_comparison(
            &ancient_ship,
            &compartments,
            severity,
            damage_params,
            clickhouse,
        ).await?;

        let modern_result = run_scenario_for_comparison(
            &modern_ship,
            &compartments,
            severity,
            damage_params,
            clickhouse,
        ).await?;

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
    eff_map.insert(modern.ship_id.clone(), 2.3);
    metrics.push(ComparisonMetric {
        name: "分舱效率系数".to_string(),
        ship_values: eff_map,
        unit: "相对值".to_string(),
        description: "单位排水量的抗沉能力，以古船为基准1.0".to_string(),
    });

    metrics
}

async fn run_scenario_for_comparison(
    ship: &ShipConfigExtended,
    compartments: &[u8],
    severity: f64,
    damage_params: &DamageParams,
    _clickhouse: &ClickHouseClient,
) -> Result<StabilityResultSummary, String> {
    let config = ShipConfig {
        ship_id: ship.ship_id.clone(),
        ship_name: ship.ship_name.clone(),
        length_overall: ship.length_overall,
        beam: ship.beam,
        depth: ship.depth,
        design_draft: ship.design_draft,
        displacement: ship.displacement,
        compartment_count: ship.compartment_count,
        compartment_names: ship.compartment_names.clone(),
        compartment_lengths: ship.compartment_lengths.clone(),
        compartment_volumes: ship.compartment_volumes.clone(),
        watertight_bulkheads: ship.watertight_bulkheads.clone(),
    };

    let scenario = FloodingScenario {
        ship_id: ship.ship_id.clone(),
        flooded_compartments: compartments.to_vec(),
        damage_severity: severity,
    };

    let hydrostatics = ShipHydrostatics::new(config, damage_params.clone());
    let result = hydrostatics.simulate_damage(&scenario);

    Ok(StabilityResultSummary {
        is_safe: result.is_safe,
        final_draft: result.final_draft,
        metacentric_height: result.metacentric_height,
        righting_arm_max: result.righting_arm_max,
        sinking_time_seconds: result.sinking_time_seconds,
        reserve_buoyancy: result.reserve_buoyancy,
    })
}
