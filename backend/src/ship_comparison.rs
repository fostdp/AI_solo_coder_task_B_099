pub use crate::design_comparator::DesignComparator;
pub use crate::era_comparator::EraComparator;
pub use crate::ship_configs::{
    get_all_ships_list, get_builtin_ship_configs, get_ship_config_extended,
    ship_config_extended_to_base,
};

use crate::design_comparator::DesignComparator;
use crate::models::*;

pub fn compare_ships(ship_ids: &[String], damage_params: &DamageParams) -> ShipComparisonResult {
    let comparator = DesignComparator::new(damage_params.clone());
    comparator.compare_ships(ship_ids)
}

pub async fn compare_eras(
    request: &EraComparisonRequest,
    damage_params: &DamageParams,
    _clickhouse: &crate::clickhouse_client::ClickHouseClient,
) -> Result<EraComparisonResult, String> {
    let comparator = EraComparator::new(damage_params.clone());
    comparator.compare_eras(request)
}

fn _generate_comparison_conclusion(
    ships: &[ShipConfigExtended],
    metrics: &[ComparisonMetric],
) -> String {
    let comparator = DesignComparator::new(DamageParams::default());
    comparator.generate_comparison_conclusion(ships, metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DamageParams;

    fn default_params() -> DamageParams {
        DamageParams::default()
    }

    #[test]
    fn test_get_builtin_ship_configs_count() {
        let ships = get_builtin_ship_configs();
        assert_eq!(ships.len(), 5);
    }

    #[test]
    fn test_compare_ships_backward_compat() {
        let ship_ids = vec![
            "quanzhou_song_001".to_string(),
            "fu_ship_001".to_string(),
        ];
        let result = compare_ships(&ship_ids, &default_params());
        assert_eq!(result.ships.len(), 2);
        assert!(!result.comparison_metrics.is_empty());
    }

    #[test]
    fn test_design_comparator_reexport() {
        let c = DesignComparator::new(default_params());
        assert_eq!(c.damage_params().gravity, 9.81);
    }

    #[test]
    fn test_era_comparator_reexport() {
        let c = EraComparator::new(default_params());
        assert_eq!(c.damage_params().gravity, 9.81);
    }
}
