use crate::model::{
    admin_model::{MissingTables, TimesRun},
    db::{self, test_is_db_setup, TABLE_NAMES},
};

use maud::{html, Markup};
use serde_json::{json, Value};

pub struct CreateTableReturn {
    pub html: Markup,
    pub times_run: Value,
    pub times_run_int: i32,
}

// Render the main page
pub async fn render_default_page() -> Markup {
    let admin_00 = render_create_table_results().await;
    html! {
        (maud::DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title { "Golf Admin Setup Page" }
                // Include htmx
                script src="https://unpkg.com/htmx.org@1.9.12" {}
                script src="static/admin00.js" {}
                link rel="stylesheet" type="text/css" href="static/styles.css";
            }
            body {
                div id="results" {}
                div id="admin-00" {
                    (admin_00)
                }
            }
        }
    }
}

async fn render_create_table_results() -> Markup {
    let are_db_tables_setup = test_is_db_setup().await.unwrap();

    let all_tables_setup = are_db_tables_setup
        .iter()
        .all(|x| x.db_last_exec_state == db::DatabaseSetupState::QueryReturnedSuccessfully);

    let mut json_data = json!([]);
    if !all_tables_setup {
        let missing_tables: Vec<_> = are_db_tables_setup
            .iter()
            .filter(|x| x.db_last_exec_state != db::DatabaseSetupState::QueryReturnedSuccessfully)
            .map(|x| json!({ "missing_table": x.table_or_function_name.clone() }))
            .collect();

        // Serialize the array of missing tables to JSON
        json_data = json!(missing_tables);
    }

    let times_run = json!({ "times_run": 0 });

    html! {
        @for dbresult in &are_db_tables_setup {
            @let message = format!("db result: {:?}, table name: {}, db err msg: {}"
                , dbresult.db_last_exec_state
                , dbresult.table_or_function_name
                , dbresult.error_message.clone().unwrap_or("".to_string())
            );
            p { (message) }
        }

        script type="application/json" id="admin00_missing_tables" {
            (json_data)
        }

        script type="application/json" id="times_run" {
            { (times_run) }
        }

        @if all_tables_setup {
            p { "All tables are setup" }
        } @else {
            button
            hx-trigger="reenablebutton from:body"
            id="create-missing-tables"
            {
                "Create missing tables"
            }
        }

        div id="create-table-results"  {}
    }
}

pub async fn create_tables(data: String, times_run: String) -> CreateTableReturn {
    let mut result = CreateTableReturn {
        html: html!(p { "No data" }),
        times_run: json!({ "times_run": 0 }),
        times_run_int: 0,
    };
    let data: Vec<MissingTables> = match serde_json::from_str(&data) {
        Ok(d) => d,
        Err(e) => {
            result.html = html! {
            p { "Invalid table data: " (e) }};

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
        .filter(|x| TABLE_NAMES.contains(&x.missing_table.as_str()))
        .collect();

    let times_run_int = times_run_from_json.times_run + 1;
    result.times_run = json!({ "times_run": times_run_int });
    result.times_run_int = times_run_int;

    result.html = html! {
        p { "You've run this function " (result.times_run) " times" }
        @for table in data {
            p { "Creating table: " (table.missing_table) }
        }
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
