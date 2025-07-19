use std::collections::HashMap;
use actix_web::{HttpResponse, web};
use crate::admin::model::admin_model::create_tables;
use crate::model::CheckType;
use maud::html;
use serde_json::json;
use super::types::CreateTableReturn;
use super::utils::parse_into_times_run;

impl CreateTableReturn {
    /// Return bit of html indicating if tables created, plus some headers to trigger htmx
    pub async fn http_response_for_create_tables(
        &mut self,
        query: web::Query<HashMap<String, String>>,
    ) -> Result<HttpResponse, Box<dyn std::error::Error>> {
        let times_run = query
            .get("times_run")
            .unwrap_or(&String::new())
            .trim()
            .to_string();

        let create_tables_html = self.create_tables(times_run).await?;

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
            table_exist_query: self.table_exist_query,
        };

        let times_run_from_json = match parse_into_times_run(&times_run) {
            Some(d) => d,
            None => {
                let str = format!("Invalid times_run data: {times_run}");
                result.html = html! {
                p { (str) }};
                return Ok(result);
            }
        };

        let times_run_int = times_run_from_json.times_run + 1;
        result.times_run = json!({ "times_run": times_run_int });
        result.times_run_int = times_run_int;

        let actual_table_creation = create_tables(&self.config_and_pool, &CheckType::Table).await;

        let message: String = match actual_table_creation {
            Ok(_x) => "Tables created successfully".to_string(),
            Err(e) => format!("Error creating tables: {e:?}"),
        };

        let actual_constraint_creation =
            create_tables(&self.config_and_pool, &CheckType::Constraint).await;
        let message2: String = match actual_constraint_creation {
            Ok(_x) => "Table constraints created successfully".to_string(),
            Err(e) => format!("Error creating tables: {e:?}"),
        };

        result.html = html! {
            p { "You've run this function " (result.times_run_int) " times." }
            p { "Creating tables output: " (message) }
            p { "Creating table constraints output: " (message2) }
        };
        Ok(result)
    }
}