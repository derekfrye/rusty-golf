use actix_web::web::{self, Data};
use actix_web::{HttpResponse, Responder};
use serde_json::json;
use std::collections::HashMap;

use crate::mvu::runtime::run_score;
use crate::mvu::score as mvu_score;
use crate::storage::SqlStorage;
use crate::view::score::chart::render_drop_down_bar_pure;
use crate::view::score::types::RefreshData;
use crate::view::score::{
    render_line_score_tables, render_summary_scores, scores_and_last_refresh_to_line_score_tables,
};

// The `implicit_hasher` lint is allowed here because the `HashMap` is created by `actix-web`
// as part of the query string parsing. We cannot control the hasher used in this case,
// and the performance impact is negligible for a small number of query parameters.
#[allow(clippy::implicit_hasher)]
pub async fn scores(
    query: web::Query<HashMap<String, String>>,
    storage: Data<SqlStorage>,
) -> impl Responder {
    let storage_ref = storage.get_ref();

    // Decode request → model
    let mut model = match mvu_score::decode_request_to_model(&query, storage_ref).await {
        Ok(m) => m,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": e.to_string()})),
    };
    // MVU: request-driven, no periodic triggers
    let _ = run_score(
        &mut model,
        mvu_score::Msg::PageLoad,
        mvu_score::Deps {
            storage: storage_ref,
        },
    )
    .await;

    if let Some(err) = model.error {
        return HttpResponse::InternalServerError().json(json!({"error": err.to_string()}));
    }

    if model.want_json {
        if let Some(data) = model.data {
            HttpResponse::Ok().json(data)
        } else {
            HttpResponse::InternalServerError().json(json!({"error": "No data in model"}))
        }
    } else if let Some(markup) = model.markup {
        HttpResponse::Ok()
            .content_type("text/html")
            .body(markup.into_string())
    } else {
        HttpResponse::InternalServerError().json(json!({"error": "No view produced"}))
    }
}

#[allow(clippy::implicit_hasher)]
pub async fn scores_summary(
    query: web::Query<HashMap<String, String>>,
    storage: Data<SqlStorage>,
) -> impl Responder {
    let storage_ref = storage.get_ref();

    // Only render when expanded=1 is explicitly requested
    let expanded = matches!(query.get("expanded").map(String::as_str), Some("1"));
    if !expanded {
        return HttpResponse::NoContent().finish();
    }

    let mut model = match mvu_score::decode_request_to_model(&query, storage_ref).await {
        Ok(m) => m,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": e.to_string()})),
    };
    model.want_json = false;
    let _ = run_score(
        &mut model,
        mvu_score::Msg::PageLoad,
        mvu_score::Deps {
            storage: storage_ref,
        },
    )
    .await;

    if let Some(err) = model.error {
        return HttpResponse::InternalServerError().json(json!({"error": err.to_string()}));
    }
    let Some(ref data) = model.data else {
        return HttpResponse::InternalServerError().json(json!({"error": "No data"}));
    };

    let summary = crate::controller::score::group_by_bettor_name_and_round(&data.score_struct);
    let markup = render_summary_scores(&summary);
    HttpResponse::Ok()
        .content_type("text/html")
        .body(markup.into_string())
}

#[allow(clippy::implicit_hasher)]
pub async fn scores_chart(
    query: web::Query<HashMap<String, String>>,
    storage: Data<SqlStorage>,
) -> impl Responder {
    let storage_ref = storage.get_ref();
    let mut model = match mvu_score::decode_request_to_model(&query, storage_ref).await {
        Ok(m) => m,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": e.to_string()})),
    };
    model.want_json = false;
    let _ = run_score(
        &mut model,
        mvu_score::Msg::PageLoad,
        mvu_score::Deps {
            storage: storage_ref,
        },
    )
    .await;
    if let Some(err) = model.error {
        return HttpResponse::InternalServerError().json(json!({"error": err.to_string()}));
    }
    let Some(ref data) = model.data else {
        return HttpResponse::InternalServerError().json(json!({"error": "No data"}));
    };
    let Some(global) = model.global_step_factor else {
        return HttpResponse::InternalServerError().json(json!({"error": "No global step factor"}));
    };
    let Some(ref factors) = model.player_step_factors else {
        return HttpResponse::InternalServerError()
            .json(json!({"error": "No player step factors"}));
    };

    let summary_scores_x =
        crate::controller::score::group_by_bettor_name_and_round(&data.score_struct);
    let detailed_scores =
        crate::controller::score::group_by_bettor_golfer_round(&data.score_struct);
    let markup = render_drop_down_bar_pure(&summary_scores_x, &detailed_scores, global, factors);
    HttpResponse::Ok()
        .content_type("text/html")
        .body(markup.into_string())
}

#[allow(clippy::implicit_hasher)]
pub async fn scores_linescore(
    query: web::Query<HashMap<String, String>>,
    storage: Data<SqlStorage>,
) -> impl Responder {
    let storage_ref = storage.get_ref();
    let mut model = match mvu_score::decode_request_to_model(&query, storage_ref).await {
        Ok(m) => m,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": e.to_string()})),
    };
    model.want_json = false;
    let _ = run_score(
        &mut model,
        mvu_score::Msg::PageLoad,
        mvu_score::Deps {
            storage: storage_ref,
        },
    )
    .await;
    if let Some(err) = model.error {
        return HttpResponse::InternalServerError().json(json!({"error": err.to_string()}));
    }
    let Some(ref data) = model.data else {
        return HttpResponse::InternalServerError().json(json!({"error": "No data"}));
    };
    let Some(ref from_db) = model.from_db_scores else {
        return HttpResponse::InternalServerError().json(json!({"error": "No DB scores"}));
    };
    let bettor_struct = scores_and_last_refresh_to_line_score_tables(from_db);
    let refresh_data = RefreshData {
        last_refresh: data.last_refresh.clone(),
        last_refresh_source: data.last_refresh_source.clone(),
    };
    let markup = render_line_score_tables(&bettor_struct, &refresh_data);
    HttpResponse::Ok()
        .content_type("text/html")
        .body(markup.into_string())
}
