use super::db_operations::get_missing_db_objects;
use super::types::CreateTableReturn;
use super::utils::get_check_type_data;
use crate::HTMX_PATH;
use crate::model::CheckType;
use maud::{Markup, html};
use serde_json::json;
use sql_middleware::middleware::ConfigAndPool;

impl CreateTableReturn {
    pub async fn new(config_and_pool: ConfigAndPool) -> Self {
        Self {
            html: html!(p { "No data" }),
            times_run: json!({ "times_run": 0 }),
            times_run_int: 0,
            config_and_pool,
            table_exist_query: include_str!("../../model/sql/schema/postgres/0x_tables_exist.sql"),
        }
    }

    // Render the main page
    pub async fn render_default_page(&mut self) -> Result<Markup, Box<dyn std::error::Error>> {
        let do_tables_exist = self.do_tables_exist(true, CheckType::Table).await?;

        Ok(html! {
            (maud::DOCTYPE)
            html {
                head {
                    meta charset="utf-8";
                    title { "Golf Admin Setup Page" }
                    // Include htmx
                    script src=(HTMX_PATH) defer {}
                    script src="static/admin01.js" defer {}
                    link rel="preload" href="static/styles.css" as="style" onload="this.rel='stylesheet'";
                }
                body {
                    div id="results" {}
                    div id="admin-01" {
                        (do_tables_exist)
                    }
                }
            }
        })
    }

    pub async fn check_if_tables_exist(&mut self) -> Result<Markup, Box<dyn std::error::Error>> {
        self.do_tables_exist(false, CheckType::Table).await
    }

    pub async fn check_if_constraints_exist(
        &mut self,
    ) -> Result<Markup, Box<dyn std::error::Error>> {
        self.do_tables_exist(false, CheckType::Constraint).await
    }

    async fn do_tables_exist(
        &mut self,
        detailed_output: bool,
        check_type: CheckType,
    ) -> Result<Markup, Box<dyn std::error::Error>> {
        let (all_objs_not_setup, json_data) =
            get_missing_db_objects(&self.config_and_pool, &check_type).await?;

        let data = get_check_type_data(&check_type);

        let missing_item_id = data.missing_item_id;
        let all_items_setup_p = data.all_items_setup_p;
        let all_items_not_setup_p = data.all_items_not_setup_p;
        let create_missing_obj_id = data.create_missing_obj_id;
        let create_missing_obj_p = data.create_missing_obj_p;
        let create_obj_results_id = data.create_obj_results_id;

        let times_run = json!({ "times_run": 0 });

        Ok(html! {
            @if detailed_output {
                @for dbresult in &all_objs_not_setup {
                    @let message = format!("missing table name: {dbresult}"

                    );
                    p { (message) }
                }

                script type="application/json" id=(missing_item_id) {
                    (json_data)
                }

                script type="application/json" id="times_run" {
                    { (times_run) }
                }
            }

            @if all_objs_not_setup.is_empty() {
                p { (all_items_setup_p) }
            } @else if detailed_output {
             button
                data-hx-trigger="reenablebutton from:body"
                id=(create_missing_obj_id)
                {
                    (create_missing_obj_p)
                }
                div id=(create_obj_results_id)  {}
            }
            @else {
                p { (all_items_not_setup_p) }
            }
        })
    }
}
