use std::collections::HashMap;

use actix_web::{App, test, web::Data};
use serde_json::Value;

use rusty_golf::controller::score::scores;

mod common;

#[actix_web::test]
async fn test1_scores_endpoint() -> Result<(), Box<dyn std::error::Error>> {
    let test_ctx = common::setup_test_context(include_str!("test1.sql"))
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let app = test::init_service(
        App::new()
            .app_data(Data::new(test_ctx.config_and_pool.clone()))
            .app_data(Data::new(test_ctx.args.clone()))
            .route("/scores", actix_web::web::get().to(scores)),
    )
    .await;

    let query_params = HashMap::from([
        ("event", "401580351".to_string()),
        ("yr", "2024".to_string()),
        ("cache", "false".to_string()),
        ("json", "true".to_string()),
    ]);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/scores?event={}&yr={}&cache={}&json={}",
            query_params["event"], query_params["yr"], query_params["cache"], query_params["json"]
        ))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Unexpected status from /scores: {}",
        resp.status()
    );

    let body: Value = test::read_body_json(resp).await;
    assert!(
        body.is_object(),
        "Response is not a JSON object; got {body:?}"
    );

    let bettor_struct = body
        .get("bettor_struct")
        .and_then(|v| v.as_array())
        .expect("Response JSON does not contain 'bettor_struct' array");
    assert_eq!(
        bettor_struct.len(),
        5,
        "Unexpected number of bettors returned"
    );

    let reference_result: Value = serde_json::from_str(include_str!("test1_expected_output.json"))?;
    let reference_array = reference_result
        .get("bettor_struct")
        .and_then(|v| v.as_array())
        .expect("Reference JSON missing bettor_struct");

    for bettor in bettor_struct {
        let bettor_name = bettor
            .get("bettor_name")
            .and_then(Value::as_str)
            .expect("Score entry missing 'bettor_name'");
        let total_score = bettor
            .get("total_score")
            .and_then(Value::as_i64)
            .expect("Score entry missing 'total_score'");
        let scoreboard_position = bettor
            .get("scoreboard_position")
            .and_then(Value::as_i64)
            .expect("Score entry missing 'scoreboard_position'");
        let scoreboard_position_name = bettor
            .get("scoreboard_position_name")
            .and_then(Value::as_str)
            .expect("Score entry missing 'scoreboard_position_name'");

        let reference_bettor = reference_array
            .iter()
            .find(|candidate| {
                candidate.get("bettor_name").and_then(Value::as_str) == Some(bettor_name)
            })
            .unwrap_or_else(|| panic!("Reference JSON missing bettor '{bettor_name}'"));

        assert_eq!(
            total_score,
            reference_bettor
                .get("total_score")
                .and_then(Value::as_i64)
                .expect("Reference entry missing total_score"),
            "Total score mismatch for bettor '{bettor_name}'"
        );

        assert_eq!(
            scoreboard_position,
            reference_bettor
                .get("scoreboard_position")
                .and_then(Value::as_i64)
                .expect("Reference entry missing scoreboard_position"),
            "Scoreboard position mismatch for bettor '{bettor_name}'"
        );

        assert_eq!(
            scoreboard_position_name,
            reference_bettor
                .get("scoreboard_position_name")
                .and_then(Value::as_str)
                .expect("Reference entry missing scoreboard_position_name"),
            "Scoreboard position name mismatch for bettor '{bettor_name}'"
        );
    }

    Ok(())
}
