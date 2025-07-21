use crate::admin::model::admin_model::test_is_db_setup;
use crate::model::CheckType;
use serde_json::json;
use sql_middleware::middleware::ConfigAndPool;

/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn get_missing_db_objects(
    config_and_pool: &ConfigAndPool,
    check_type: &CheckType,
) -> Result<(Vec<String>, serde_json::Value), Box<dyn std::error::Error>> {
    let db_obj_setup_state = test_is_db_setup(config_and_pool, check_type).await?;

    let all_objs_not_setup: Vec<String> = {
        db_obj_setup_state
            .iter()
            .filter(|x| !x.get("ex").and_then(|v| v.as_bool()).unwrap_or(&false))
            .map(|x| {
                x.get("tbl")
                    .ok_or("No tbl")
                    .and_then(|v| v.as_text().ok_or("Not a string"))
                    .map(ToString::to_string)
            })
            .collect::<Result<Vec<String>, &str>>()?
    };

    let mut json_data = json!([]);

    // for the objs not setup, we need to share that back to the web page via json
    if !all_objs_not_setup.is_empty() {
        let list_of_missing_objs: Vec<_> = all_objs_not_setup
            .iter()
            .map(|x| json!({ "missing_object": x }))
            .collect();

        // Serialize the array of missing tables to JSON
        json_data = json!(list_of_missing_objs);
    }

    Ok((all_objs_not_setup, json_data))
}
