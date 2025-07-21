use crate::admin::model::admin_model::{AdminPage, AlphaNum14};
use crate::model::CheckType;

use actix_web::{HttpResponse, web};
use maud::PreEscaped;
use sql_middleware::middleware::ConfigAndPool;
use std::{collections::HashMap, env};

use super::view::admin01_tables::CreateTableReturn;

#[derive(Debug, Clone)]
pub struct AdminRouter {
    // pub create_table_return: CreateTableReturn,
}
impl Default for AdminRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl AdminRouter {
    const UNAUTHORIZED_BODY: &str = r"
    <!DOCTYPE html>
    <html>
    <head>
        <title>Unauthorized</title>
        <style>
            body { font-family: Arial, sans-serif; background-color: #f4f4f4; text-align: center; padding: 50px; }
            h1 { color: #333; }
            p { color: #666; }
        </style>
    </head>
    <body>
        <h1>401 Unauthorized</h1>
    </body>
    </html>
    ";

    const INVALID_ADMIN_BODY: &str = r"
    <!DOCTYPE html>
    <html>
    <head>
        <title>Invalid page</title>
        <style>
            body { font-family: Arial, sans-serif; background-color: #f4f4f4; text-align: center; padding: 50px; }
            h1 { color: #333; }
            p { color: #666; }
        </style>
    </head>
    <body>
        <h1>Sorry, we can't find that admin page.</h1>
        <p>Check your <pre>p</pre> parameter.</p>
    </body>
    </html>
    ";
    #[must_use]
    pub fn new() -> Self {
        Self {
            // create_table_return: CreateTableReturn::new(db),
        }
    }
    /// # Errors
    ///
    /// Will return `Err` if the database query fails
    pub async fn router(
        &mut self,
        query: web::Query<HashMap<String, String>>,
        config_and_pool: ConfigAndPool,
    ) -> Result<HttpResponse, actix_web::Error> {
        let token_str = query
            .get("token")
            .unwrap_or(&String::new())
            .trim()
            .to_string();

        // let mut token: AlphaNum14 = AlphaNum14::default();
        let token: AlphaNum14 = AlphaNum14::parse(&token_str).unwrap_or_default();
        let admin_page = AdminPage::parse(
            query
                .get("p")
                .unwrap_or(&String::new())
                .trim()
                .to_string()
                .as_str(),
        );

        let returned_html_content: PreEscaped<String>;
        // the token determines authorized access or not
        // see README if you're trying to figure out how to set this
        if let Ok(env_token) = env::var("TOKEN") {
            // unauthorized
            if env_token != token.value() {
                returned_html_content = PreEscaped(Self::UNAUTHORIZED_BODY.to_string());
                return Ok(HttpResponse::Ok()
                    .content_type("text/html")
                    .body(returned_html_content.into_string()));
            }
        }

        let mut cr = CreateTableReturn::new(config_and_pool).await;

        // default page if p is empty in query string
        if admin_page == AdminPage::Landing {
            returned_html_content =
                crate::admin::view::admin00_landing::render_default_page(token).await;
        } else if admin_page == AdminPage::ZeroX {
            if query.contains_key("data") {
                returned_html_content = crate::admin::view::admin0x::render_received_data(query);
            } else {
                returned_html_content = crate::admin::view::admin0x::render_default_page().await;
            }
        } else if admin_page == AdminPage::TablesAndConstraints {
            if query.contains_key("admin01_missing_tables") {
                // we have special headers in this response
                return Ok(cr.http_response_for_create_tables(query).await?);
            } else if query.contains_key("from_landing_page_tables") {
                // we're on the main landing page, and checking if the db tables exist
                returned_html_content = cr
                    .check_if_db_objects_exist(false, CheckType::Table)
                    .await?;
            } else if query.contains_key("from_landing_page_constraints") {
                // we're on the main landing page, and checking if the db constraints exist
                returned_html_content = cr
                    .check_if_db_objects_exist(false, CheckType::Constraint)
                    .await?;
            } else {
                // admin01_missing_tables is populated by js when user already on this page
                // so if empty it means we need to render default page
                returned_html_content = cr.render_default_page().await?;
            }
        } else {
            returned_html_content = PreEscaped(Self::INVALID_ADMIN_BODY.to_string());
        }

        Ok(HttpResponse::Ok()
            .content_type("text/html")
            .body(returned_html_content.into_string()))
    }
}
