use crate::model::{ admin_model::MissingTables, db::{ self, test_is_db_setup } };

use maud::{ html, Markup };
use serde_json::json;

// Render the main page
pub async fn render_default_page() -> Markup {
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

pub async fn create_tables(data: String) -> Markup {
    let data: Vec<MissingTables> = match serde_json::from_str(&data) {
        Ok(d) => d,
        Err(e) => {
            return html! {
                p { "Invalid data: " (e) }
            };
        }
    };
    // let admin_00 = crate::controller::templates::admin::admin00::render_default_page().await;
    html! {
        @for table in data {
            p { "Creating table: " (table.missing_table) }
        }
    }
}
