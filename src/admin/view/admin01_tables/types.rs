use maud::Markup;
use serde_json::Value;
use sql_middleware::middleware::ConfigAndPool;

#[derive(Debug, Clone)]
pub struct CreateTableReturn {
    pub html: Markup,
    pub times_run: Value,
    pub times_run_int: i32,
    pub config_and_pool: ConfigAndPool,
    pub table_exist_query: &'static str,
}

pub struct CheckTypeData<'a> {
    pub missing_item_id: &'a str,
    pub all_items_setup_p: &'a str,
    pub all_items_not_setup_p: String,
    pub create_missing_obj_id: &'a str,
    pub create_missing_obj_p: &'a str,
    pub create_obj_results_id: &'a str,
}
