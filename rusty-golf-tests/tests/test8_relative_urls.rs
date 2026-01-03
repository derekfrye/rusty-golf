use actix_web::{App, test, web};
use scraper::{Html, Selector};

use rusty_golf::controller::score::http_handlers::{
    scores_chart, scores_linescore, scores_summary,
};
use rusty_golf::controller::score::scores;

mod common;

#[actix_web::test]
async fn test_scores_hx_routes_are_relative() -> Result<(), Box<dyn std::error::Error>> {
    let test_ctx = common::setup_test_context(include_str!("test1.sql"))
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let storage = rusty_golf::storage::SqlStorage::new(test_ctx.config_and_pool.clone());
    let app = test::init_service(
        App::new().app_data(web::Data::new(storage)).service(
            web::scope("/golf")
                .route("/scores", web::get().to(scores))
                .route("/scores/summary", web::get().to(scores_summary))
                .route("/scores/chart", web::get().to(scores_chart))
                .route("/scores/linescore", web::get().to(scores_linescore)),
        ),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/golf/scores?event=401580351&yr=2024&cache=1&expanded=0")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Unexpected status from /golf/scores: {}",
        resp.status()
    );

    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).expect("scores response should be UTF-8");

    let document = Html::parse_fragment(&body_str);
    let selector = Selector::parse("[hx-get]").expect("valid selector");
    let mut hx_targets = Vec::new();
    for element in document.select(&selector) {
        if let Some(value) = element.value().attr("hx-get") {
            assert!(
                !value.starts_with('/'),
                "hx-get attribute should be relative but found '{value}'"
            );
            hx_targets.push(value.to_string());
        }
    }

    assert!(
        !hx_targets.is_empty(),
        "Expected at least one hx-get attribute in scores markup"
    );

    for hx in hx_targets {
        let follow_req = test::TestRequest::get()
            .uri(&format!("/golf/{hx}"))
            .to_request();
        let follow_resp = test::call_service(&app, follow_req).await;
        assert!(
            follow_resp.status().is_success(),
            "Follow-up hx-get '{hx}' returned status {}",
            follow_resp.status()
        );
    }

    Ok(())
}
