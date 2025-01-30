use std::collections::HashMap;

use crate::{
    admin::model::admin_model::{create_tables, test_is_db_setup, TimesRun},
    HTMX_PATH,
};
use actix_web::{web, HttpResponse};
use maud::{html, Markup};
use serde_json::{json, Value};
use sqlx_middleware::middleware::{CheckType, ConfigAndPool};

#[derive(Debug, Clone)]
pub struct CreateTableReturn {
    pub html: Markup,
    pub times_run: Value,
    pub times_run_int: i32,
    pub config_and_pool: ConfigAndPool,
    // tables: Vec<DatabaseItem>,
    table_exist_query: &'static str,
}
impl CreateTableReturn {
    pub async fn new(config_and_pool: ConfigAndPool) -> Self {
        // let mut z = Vec::new();
        // for x in TABLES_AND_DDL {
        //     let dbitem = DatabaseItem::Table(DatabaseTable {
        //         table_name: x.0.to_string(),
        //         ddl: x.1.to_string(),
        //     });
        //     z.push(dbitem);
        // }

        Self {
            html: html!(p { "No data" }),
            times_run: json!({ "times_run": 0 }),
            times_run_int: 0,
            config_and_pool,
            // tables: z,
            table_exist_query: include_str!("../model/sql/schema/postgres/0x_tables_exist.sql"),
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
        // let query = include_str!("../model/sql/schema/0x_tables_exist.sql");

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
        check_type: CheckType, // query: &str,
    ) -> Result<Markup, Box<dyn std::error::Error>> {
        let db_obj_setup_state = test_is_db_setup(&self.config_and_pool, &check_type).await?;

        let all_objs_not_setup: Vec<&str> = {
            db_obj_setup_state
                .iter()
                .filter(|x| *x.get("ex").and_then(|v| v.as_bool()).unwrap_or(&false) != true)
                .map(|x| {
                    x.get("tbl")
                        .ok_or("No tbl")
                        .and_then(|v| v.as_text().ok_or("Not a string"))
                })
                .collect::<Result<Vec<&str>, &str>>()?
        };

        let mut json_data = json!([]);

        // for the objs not setup, we need to share that back to the web page via json
        if all_objs_not_setup.len() > 0 {
            let list_of_missing_objs: Vec<_> = all_objs_not_setup
                .clone()
                .into_iter()
                .map(|x| json!({ "missing_object": x }))
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

        fn get_check_type_data<'a>(check_type: &CheckType) -> CheckTypeData<'a> {
            match check_type {
                CheckType::Table => CheckTypeData {
                    missing_item_id: "admin01_missing_tables",
                    all_items_setup_p: "All tables are setup.",
                    all_items_not_setup_p: format!("Not all tables are setup."),
                    create_missing_obj_id: "create-missing-tables",
                    create_missing_obj_p: "Create missing tables",
                    create_obj_results_id: "create-table-results",
                },
                CheckType::Constraint => CheckTypeData {
                    missing_item_id: "admin01_missing_constraints",
                    all_items_setup_p: "All constraints are setup.",
                    all_items_not_setup_p: format!("Not all constraints are setup."),
                    create_missing_obj_id: "create-missing-constraints",
                    create_missing_obj_p: "Create missing constraints",
                    create_obj_results_id: "create-constraint-results",
                },
            }
        }

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
                    @let message = format!("missing table name: {}"
                        , dbresult

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

            @if all_objs_not_setup.len() == 0 {
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

    /// Return bit of html indicating if tables created, plus some headers to trigger htmx
    pub async fn http_response_for_create_tables(
        &mut self,
        query: web::Query<HashMap<String, String>>,
    ) -> Result<HttpResponse, Box<dyn std::error::Error>> {
        // let missing_tables = query
        //     .get("admin01_missing_tables")
        //     .unwrap_or(&String::new())
        //     .trim()
        //     .to_string();
        let times_run = query
            .get("times_run")
            .unwrap_or(&String::new())
            .trim()
            .to_string();

        let create_tables_html = self.create_tables(times_run).await?;

        // dbg!("markup_from_admin", &markup_from_admin.times_run_int);

        let header = json!({"reenablebutton": "1",
               "times_run": create_tables_html.times_run_int
        });

        Ok(HttpResponse::Ok()
            .content_type("text/html")
            // Add the HX-Trigger header, this tells the create table button to reenable (based on a fn in js)
            .insert_header(("HX-Trigger", header.to_string()))
            .body(create_tables_html.html.into_string()))
    }

    /// try creating the tables and return small bit of html via htmx for outcome
    async fn create_tables(
        &mut self,
        times_run: String,
    ) -> Result<CreateTableReturn, Box<dyn std::error::Error>> {
        let mut result = CreateTableReturn {
            html: html!(p { "No data" }),
            times_run: json!({ "times_run": 0 }),
            times_run_int: 0,
            config_and_pool: self.config_and_pool.clone(),
            // tables: self.tables.clone(),
            table_exist_query: self.table_exist_query,
        };
        // let data: Vec<MissingDbObjects> = match serde_json::from_str(&data) {
        //     Ok(d) => d,
        //     Err(e) => {
        //         let message = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
        //         result.html = html! {
        //         p { "Invalid table data: " (message) }};

        //         return Ok( result);
        //     }
        // };

        let times_run_from_json = match Self::parse_into_times_run(&times_run) {
            Some(d) => d,
            None => {
                let str = format!("Invalid times_run data: {}", times_run);
                result.html = html! {
                p { (str) }};
                return Ok(result);
            }
        };

        let times_run_int = times_run_from_json.times_run + 1;
        result.times_run = json!({ "times_run": times_run_int });
        result.times_run_int = times_run_int;

        let actual_table_creation = create_tables(&self.config_and_pool, &CheckType::Table).await;
        // .create_tables(data.clone(), CheckType::Table, TABLES_AND_DDL)

        let message: String;
        match actual_table_creation {
            Ok(_x) => {
                message = "Tables created successfully".to_string();
            }
            Err(e) => {
                message = format!("Error creating tables: {:?}", e);
            }
        }

        let actual_constraint_creation =
            create_tables(&self.config_and_pool, &CheckType::Constraint).await;
        let message2: String;
        match actual_constraint_creation {
            Ok(_x) => {
                message2 = "Table constraints created successfully".to_string();
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
        Ok(result)
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
