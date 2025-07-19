use super::types::CheckTypeData;
use crate::admin::model::admin_model::TimesRun;
use crate::model::CheckType;

pub fn parse_into_times_run(input: &str) -> Option<TimesRun> {
    match serde_json::from_str::<TimesRun>(input) {
        Ok(single_run) => Some(single_run),
        Err(_) => {
            // If single parse fails, try to parse as Vec<TimesRun>
            match serde_json::from_str::<Vec<TimesRun>>(input) {
                Ok(mut runs) => {
                    // If the Vec is not empty, return the first element
                    if !runs.is_empty() {
                        Some(runs.remove(0))
                    } else {
                        None
                    }
                }
                Err(_) => None, // If both deserializations fail, return None
            }
        }
    }
}

pub fn get_check_type_data(check_type: &CheckType) -> CheckTypeData {
    match check_type {
        CheckType::Table => CheckTypeData {
            missing_item_id: "admin01_missing_tables",
            all_items_setup_p: "All tables are setup.",
            all_items_not_setup_p: "Not all tables are setup.".to_string(),
            create_missing_obj_id: "create-missing-tables",
            create_missing_obj_p: "Create missing tables",
            create_obj_results_id: "create-table-results",
        },
        CheckType::Constraint => CheckTypeData {
            missing_item_id: "admin01_missing_constraints",
            all_items_setup_p: "All constraints are setup.",
            all_items_not_setup_p: "Not all constraints are setup.".to_string(),
            create_missing_obj_id: "create-missing-constraints",
            create_missing_obj_p: "Create missing constraints",
            create_obj_results_id: "create-constraint-results",
        },
    }
}
