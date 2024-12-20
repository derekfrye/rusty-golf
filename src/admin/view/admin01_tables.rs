use std::collections::HashMap;

use crate::{
    admin::model::admin_model::{MissingTables, TimesRun},
    db::{self, test_is_db_setup, CheckType, TABLES_AND_DDL},
    HTMX_PATH,
};

use actix_web::{web, HttpResponse};
use maud::{html, Markup};
use serde_json::{json, Value};

pub struct CreateTableReturn {
    pub html: Markup,
    pub times_run: Value,
    pub times_run_int: i32,
}

// Render the main page
pub async fn render_default_page() -> Markup {
    let do_tables_exist = do_tables_exist(true).await;

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

pub async fn check_if_tables_exist() -> Markup {
    let do_tables_exist = do_tables_exist(false).await;
    do_tables_exist
}

async fn do_tables_exist(detailed_output: bool, check_type: CheckType ) -> Markup {
    let are_db_tables_setup = test_is_db_setup().await.unwrap();

    let all_tables_setup = are_db_tables_setup
        .iter()
        .all(|x| x.db_last_exec_state == db::DatabaseSetupState::QueryReturnedSuccessfully);

    let mut json_data = json!([]);
    let mut last_message = String::new();
    if !all_tables_setup {
        let missing_tables = are_db_tables_setup
            .iter()
            .filter(|x| x.db_last_exec_state != db::DatabaseSetupState::QueryReturnedSuccessfully);

        match missing_tables.clone().into_iter().last() {
            Some(x) => {
                last_message = x.error_message.clone().unwrap_or("".to_string());
            }
            None => {}
        }

        let list_of_missing_tables: Vec<_> = missing_tables
            .map(|x| json!({ "missing_table": x.table_or_function_name.clone() }))
            .collect();

        // Serialize the array of missing tables to JSON
        json_data = json!(list_of_missing_tables);
    }

    let times_run = json!({ "times_run": 0 });

    html! {
        @if detailed_output {
            @for dbresult in are_db_tables_setup.iter().filter(|x| x.db_last_exec_state != db::DatabaseSetupState::QueryReturnedSuccessfully) {
                @let message = format!("db result: {:?}, table name: {}, db err msg: {}"
                    , dbresult.db_last_exec_state
                    , dbresult.table_or_function_name
                    , dbresult.error_message.clone().unwrap_or("".to_string())
                );
                p { (message) }
            }

            script type="application/json" id="admin01_missing_tables" {
                (json_data)
            }

            script type="application/json" id="times_run" {
                { (times_run) }
            }
        }

        @if all_tables_setup {
            p { "All tables are setup." }
        } @else {
            @if detailed_output {
             button
                data-hx-trigger="reenablebutton from:body"
                id="create-missing-tables"
                {
                    "Create missing tables"
                }
                div id="create-table-results"  {}
            }
            @else {
                p { "Not all tables are setup. Last error: " (last_message) }
            }
        }
    }
}

/// Return bit of html indicating if tables created, plus some headers to trigger htmx
pub async fn http_response_for_create_tables(
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

    let create_tables_html = create_tables(missing_tables, times_run).await;

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
async fn create_tables(data: String, times_run: String) -> CreateTableReturn {
    let mut result = CreateTableReturn {
        html: html!(p { "No data" }),
        times_run: json!({ "times_run": 0 }),
        times_run_int: 0,
    };
    let data: Vec<MissingTables> = match serde_json::from_str(&data) {
        Ok(d) => d,
        Err(e) => {
            let message = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
            result.html = html! {
            p { "Invalid table data: " (message) }};

            return result;
        }
    };

    let times_run_from_json = match parse_into_times_run(&times_run) {
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
    let data: Vec<MissingTables> = data
        .into_iter()
        .filter(|x| {
            TABLES_AND_DDL
                .iter()
                .map(|x| x.0)
                .collect::<Vec<_>>()
                .contains(&x.missing_table.as_str())
        })
        .collect();

    let times_run_int = times_run_from_json.times_run + 1;
    result.times_run = json!({ "times_run": times_run_int });
    result.times_run_int = times_run_int;

    let actual_table_creation = db::create_tables(data.clone(), db::CheckType::Table).await;

    let message: String;
    match actual_table_creation {
        Ok(x) => {
            if x.db_last_exec_state == db::DatabaseSetupState::QueryReturnedSuccessfully {
                message = "Tables created successfully".to_string();
            } else {
                message = format!(
                    "Tables created, but with errors. {}, {}",
                    x.error_message.unwrap_or("".to_string()),
                    x.table_or_function_name
                );
            }
        }
        Err(e) => {
            message = format!("Error creating tables: {:?}", e);
        }
    }

    let actual_constraint_creation =
        db::create_tables(data.clone(), db::CheckType::Constraint).await;
    let message2: String;
    match actual_constraint_creation {
        Ok(x) => {
            if x.db_last_exec_state == db::DatabaseSetupState::QueryReturnedSuccessfully {
                message2 = "Table constraints created successfully".to_string();
            } else {
                message2 = format!(
                    "Table constraints created, but with errors. {}, {}",
                    x.error_message.unwrap_or("".to_string()),
                    x.table_or_function_name
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
