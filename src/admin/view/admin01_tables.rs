use std::collections::HashMap;

use crate::{
    admin::model::admin_model::{MissingDbObjects, TimesRun}, model::{CheckType, TABLES_AND_DDL, TABLES_CONSTRAINT_TYPE_CONSTRAINT_NAME_AND_DDL}, HTMX_PATH
};
use sqlx_middleware::db::{self, DatabaseSetupState, Db, };
use actix_web::{web, HttpResponse};
use maud::{html, Markup};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct CreateTableReturn {
    pub html: Markup,
    pub times_run: Value,
    pub times_run_int: i32,
    pub db: Db,
}
impl CreateTableReturn {
    pub fn new(db: Db) -> Self {
        Self {
            html: html!(p { "No data" }),
            times_run: json!({ "times_run": 0 }),
            times_run_int: 0,
            db,
        }
    }

    // Render the main page
    pub async fn render_default_page(&mut self) -> Markup {
        let do_tables_exist = self.do_tables_exist(true, CheckType::Table).await;

        html! {
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
        }
    }

    pub async fn check_if_tables_exist(&mut self) -> Markup {
        let do_tables_exist = self.do_tables_exist(false, CheckType::Table).await;
        do_tables_exist
    }

    pub async fn check_if_constraints_exist(&mut self) -> Markup {
        let do_tables_exist = self.do_tables_exist(false, CheckType::Constraint).await;
        do_tables_exist
    }

    async fn do_tables_exist(&mut self, detailed_output: bool, check_type: CheckType) -> Markup {
        let db_obj_setup_state = self. db.test_is_db_setup(&check_type).await.unwrap();

        let all_objs_setup_successfully = db_obj_setup_state
            .iter()
            .all(|x| x.db_last_exec_state == db::DatabaseSetupState::QueryReturnedSuccessfully);

        let mut json_data = json!([]);
        let mut last_message = String::new();
        if !all_objs_setup_successfully {
            let missing_objs = db_obj_setup_state.iter().filter(|x| {
                x.db_last_exec_state != db::DatabaseSetupState::QueryReturnedSuccessfully
            });

            match missing_objs.clone().into_iter().last() {
                Some(x) => {
                    last_message = x.error_message.clone().unwrap_or("".to_string());
                }
                None => {}
            }

            let list_of_missing_objs: Vec<_> = missing_objs
                .map(|x| json!({ "missing_object": x.db_object_name.clone() }))
                .collect();

            // Serialize the array of missing tables to JSON
            json_data = json!(list_of_missing_objs);
        }

        struct CheckTypeData<'a> {
            missing_item_id: &'a str,
            all_items_setup_p: &'a str,
            all_items_not_setup_p: String,
            create_missing_obj_id: &'a str,
            create_missing_obj_p: &'a str,
            create_obj_results_id: &'a str,
        }

        fn get_check_type_data<'a>(
            check_type: &CheckType,
            last_message: &str,
        ) -> CheckTypeData<'a> {
            match check_type {
                CheckType::Table => CheckTypeData {
                    missing_item_id: "admin01_missing_tables",
                    all_items_setup_p: "All tables are setup.",
                    all_items_not_setup_p: format!(
                        "Not all tables are setup. Last error: {}",
                        last_message
                    ),
                    create_missing_obj_id: "create-missing-tables",
                    create_missing_obj_p: "Create missing tables",
                    create_obj_results_id: "create-table-results",
                },
                CheckType::Constraint => CheckTypeData {
                    missing_item_id: "admin01_missing_constraints",
                    all_items_setup_p: "All constraints are setup.",
                    all_items_not_setup_p: format!(
                        "Not all constraints are setup. Last error: {}",
                        last_message
                    ),
                    create_missing_obj_id: "create-missing-constraints",
                    create_missing_obj_p: "Create missing constraints",
                    create_obj_results_id: "create-constraint-results",
                },
            }
        }

        let data = get_check_type_data(&check_type, &last_message);

        let missing_item_id = data.missing_item_id;
        let all_items_setup_p = data.all_items_setup_p;
        let all_items_not_setup_p = data.all_items_not_setup_p;
        let create_missing_obj_id = data.create_missing_obj_id;
        let create_missing_obj_p = data.create_missing_obj_p;
        let create_obj_results_id = data.create_obj_results_id;

        let times_run = json!({ "times_run": 0 });

        html! {
            @if detailed_output {
                @for dbresult in db_obj_setup_state.iter().filter(|x| x.db_last_exec_state != db::DatabaseSetupState::QueryReturnedSuccessfully) {
                    @let message = format!("db result: {:?}, table name: {}, db err msg: {}"
                        , dbresult.db_last_exec_state
                        , dbresult.db_object_name
                        , dbresult.error_message.clone().unwrap_or("".to_string())
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

            @if all_objs_setup_successfully {
                p { (all_items_setup_p) }
            } @else {
                @if detailed_output {
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
            }
        }
    }

    /// Return bit of html indicating if tables created, plus some headers to trigger htmx
    pub async fn http_response_for_create_tables(
        &mut self,
        query: web::Query<HashMap<String, String>>,
    ) -> HttpResponse {
        let missing_tables = query
            .get("admin01_missing_tables")
            .unwrap_or(&String::new())
            .trim()
            .to_string();
        let times_run = query
            .get("times_run")
            .unwrap_or(&String::new())
            .trim()
            .to_string();

        let create_tables_html = self.create_tables(missing_tables, times_run).await;

        // dbg!("markup_from_admin", &markup_from_admin.times_run_int);

        let header = json!({"reenablebutton": "1",
               "times_run": create_tables_html.times_run_int
        });

        HttpResponse::Ok()
            .content_type("text/html")
            // Add the HX-Trigger header, this tells the create table button to reenable (based on a fn in js)
            .insert_header(("HX-Trigger", header.to_string()))
            .body(create_tables_html.html.into_string())
    }

    /// try creating the tables and return small bit of html for outcome
    async fn create_tables(&mut self, data: String, times_run: String) -> CreateTableReturn {
        let mut result = CreateTableReturn {
            html: html!(p { "No data" }),
            times_run: json!({ "times_run": 0 }),
            times_run_int: 0,
            db: self.db.clone(),
        };
        let data: Vec<MissingDbObjects> = match serde_json::from_str(&data) {
            Ok(d) => d,
            Err(e) => {
                let message = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
                result.html = html! {
                p { "Invalid table data: " (message) }};

                return result;
            }
        };

        let times_run_from_json = match Self::parse_into_times_run(&times_run) {
            Some(d) => d,
            None => {
                let str = format!("Invalid times_run data: {}", times_run);
                result.html = html! {
                p { (str) }};
                return result;
            }
        };

        // data validation: we only want to create tables where we match on table names
        // otherwise who knows wth we're creating in our db
        let data: Vec<MissingDbObjects> = data
            .into_iter()
            .filter(|x| {
                TABLES_AND_DDL
                    .iter()
                    .map(|x| x.0)
                    .collect::<Vec<_>>()
                    .contains(&x.missing_object.as_str())
            })
            .collect();

        let times_run_int = times_run_from_json.times_run + 1;
        result.times_run = json!({ "times_run": times_run_int });
        result.times_run_int = times_run_int;

        let actual_table_creation = self
            .db
            .create_tables(data.clone(), CheckType::Table, TABLES_AND_DDL)
            .await;

        let message: String;
        match actual_table_creation {
            Ok(x) => {
                if x.db_last_exec_state == DatabaseSetupState::QueryReturnedSuccessfully {
                    message = "Tables created successfully".to_string();
                } else {
                    message = format!(
                        "Tables created, but with errors. {}, {}",
                        x.error_message.unwrap_or("".to_string()),
                        x.db_object_name
                    );
                }
            }
            Err(e) => {
                message = format!("Error creating tables: {:?}", e);
            }
        }

        let actual_constraint_creation = self
            .db
            .create_tables(
                data.clone(),
                CheckType::Constraint,
                TABLES_CONSTRAINT_TYPE_CONSTRAINT_NAME_AND_DDL,
            )
            .await;
        let message2: String;
        match actual_constraint_creation {
            Ok(x) => {
                if x.db_last_exec_state == DatabaseSetupState::QueryReturnedSuccessfully {
                    message2 = "Table constraints created successfully".to_string();
                } else {
                    message2 = format!(
                        "Table constraints created, but with errors. {}, {}",
                        x.error_message.unwrap_or("".to_string()),
                        x.db_object_name
                    );
                }
            }
            Err(e) => {
                message2 = format!("Error creating tables: {:?}", e);
            }
        }

        result.html = html! {
            p { "You've run this function " (result.times_run_int) " times." }
            p { "Creating tables output: " (message) }
            p { "Creating table constraints output: " (message2) }
        };
        result
    }

    fn parse_into_times_run(input: &str) -> Option<TimesRun> {
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
}
