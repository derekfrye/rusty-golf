use crate::model::db::{self, test_is_db_setup};

use maud::{html, Markup};
use serde_json::json;

// Render the main page
pub async fn render_default_page() -> Markup {
    let are_db_tables_setup = test_is_db_setup().await.unwrap();

    let all_tables_not_setup = are_db_tables_setup
        .iter()
        .all(|x| x.db_last_exec_state != db::DatabaseSetupState::QueryReturnedSuccessfully);

    // Serialize the result to JSON
    let json_data = json!({ "missing_table": all_tables_not_setup });

    html! {
        loop {
            @for dbresult in &are_db_tables_setup {
                @let message = format!("db result: {:?}, table name: {}, db err msg: {}"
                    , dbresult.db_last_exec_state
                    , dbresult.table_or_function_name
                    , dbresult.error_message.clone().unwrap_or("".to_string())
                );
                p { (message) }
            }
        }

        script type="application/json" id="db-status" {
            (json_data)
        }

        @if are_db_tables_setup.iter().all(|x| x.db_last_exec_state == db::DatabaseSetupState::QueryReturnedSuccessfully) {
            p { "All tables are setup" }
        } else {
            p { "Not all tables are setup" }
        }
    }
}
