#[cfg(target_arch = "wasm32")]
pub mod storage;

pub use rusty_golf_core as core;

#[cfg(target_arch = "wasm32")]
use storage::ServerlessStorage;
#[cfg(target_arch = "wasm32")]
use worker::{event, Env, Request, Response, Result, RouteContext, Router};
#[cfg(target_arch = "wasm32")]
use {
    rusty_golf_core::model::{
        format_time_ago_for_score_view, Bettors, RefreshSource, ScoreData, ScoresAndLastRefresh,
    },
    rusty_golf_core::score::{group_by_bettor_golfer_round, group_by_bettor_name_and_round},
    rusty_golf_core::storage::Storage,
    rusty_golf_core::view::score::{
        render_drop_down_bar_pure, render_line_score_tables, render_scores_template_pure,
        render_summary_scores, scores_and_last_refresh_to_line_score_tables, RefreshData,
    },
    std::collections::HashMap,
};

#[cfg(target_arch = "wasm32")]
struct ScoreRequest {
    event_id: i32,
    year: i32,
    use_cache: bool,
    want_json: bool,
    expanded: bool,
}

#[cfg(target_arch = "wasm32")]
struct ScoreContext {
    data: ScoreData,
    from_db_scores: ScoresAndLastRefresh,
    global_step_factor: f32,
    player_step_factors: HashMap<(i64, String), f32>,
}

#[cfg(target_arch = "wasm32")]
fn parse_query_params(req: &Request) -> Result<HashMap<String, String>> {
    let url = req
        .url()
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    Ok(url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect())
}

#[cfg(target_arch = "wasm32")]
fn parse_score_request(query: &HashMap<String, String>) -> Result<ScoreRequest> {
    let event_id = query
        .get("event")
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| worker::Error::RustError("espn event parameter is required".into()))?;
    let year = query
        .get("yr")
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| worker::Error::RustError("yr (year) parameter is required".into()))?;
    let use_cache = !matches!(query.get("cache").map(String::as_str), Some("0"));
    let want_json = match query.get("json").map(String::as_str) {
        Some("1") => true,
        Some("0") | None => false,
        Some(other) => other.parse().unwrap_or(false),
    };
    let expanded = match query.get("expanded").map(String::as_str) {
        Some("1") => true,
        Some("0") | None => false,
        Some(other) => other.parse().unwrap_or(false),
    };
    Ok(ScoreRequest {
        event_id,
        year,
        use_cache,
        want_json,
        expanded,
    })
}

#[cfg(target_arch = "wasm32")]
fn score_data_from_scores(scores: &ScoresAndLastRefresh) -> ScoreData {
    let mut totals: HashMap<String, i32> = HashMap::new();
    for golfer in &scores.score_struct {
        *totals.entry(golfer.bettor_name.clone()).or_insert(0) +=
            golfer.detailed_statistics.total_score;
    }

    let mut bettors: Vec<Bettors> = totals
        .into_iter()
        .map(|(name, total)| Bettors {
            bettor_name: name,
            total_score: total,
            scoreboard_position_name: String::new(),
            scoreboard_position: 0,
        })
        .collect();

    bettors.sort_by(|a, b| {
        a.total_score
            .cmp(&b.total_score)
            .then_with(|| a.bettor_name.cmp(&b.bettor_name))
    });

    for (i, bettor) in bettors.iter_mut().enumerate() {
        bettor.scoreboard_position = i;
        bettor.scoreboard_position_name = match i {
            0 => "TOP GOLFER".to_string(),
            1 => "FIRST LOSER".to_string(),
            2 => "MEH".to_string(),
            3 => "SEEN BETTER DAYS".to_string(),
            4 => "NOT A CHANCE".to_string(),
            _ => "WORST OF THE WORST".to_string(),
        };
    }

    let elapsed = chrono::Utc::now().naive_utc() - scores.last_refresh;
    ScoreData {
        bettor_struct: bettors,
        score_struct: scores.score_struct.clone(),
        last_refresh: format_time_ago_for_score_view(elapsed),
        last_refresh_source: scores.last_refresh_source.clone(),
    }
}

#[cfg(target_arch = "wasm32")]
async fn load_score_context(
    storage: &ServerlessStorage,
    event_id: i32,
) -> Result<ScoreContext> {
    let scores = storage
        .get_scores(event_id, RefreshSource::Espn)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let event_details = storage
        .get_event_details(event_id)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let player_step_factors = storage
        .get_player_step_factors(event_id)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let data = score_data_from_scores(&scores);
    Ok(ScoreContext {
        data,
        from_db_scores: scores,
        global_step_factor: event_details.score_view_step_factor,
        player_step_factors,
    })
}

#[cfg(target_arch = "wasm32")]
fn respond_html(body: String) -> Result<Response> {
    let mut resp = Response::ok(body)?;
    resp.headers_mut()
        .set("Content-Type", "text/html")
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    Ok(resp)
}

#[cfg(target_arch = "wasm32")]
async fn scores_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let query = parse_query_params(&req)?;
    let score_req = match parse_score_request(&query) {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 400),
    };
    let storage = ServerlessStorage::from_env(
        &ctx.env,
        ServerlessStorage::KV_BINDING,
        ServerlessStorage::R2_BINDING,
    )
    .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let context = match load_score_context(&storage, score_req.event_id).await {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 500),
    };

    if score_req.want_json {
        Response::from_json(&context.data)
    } else {
        let bettor_struct =
            scores_and_last_refresh_to_line_score_tables(&context.from_db_scores);
        let markup = render_scores_template_pure(
            &context.data,
            score_req.expanded,
            &bettor_struct,
            context.global_step_factor,
            &context.player_step_factors,
            score_req.event_id,
            score_req.year,
            score_req.use_cache,
        );
        respond_html(markup.into_string())
    }
}

#[cfg(target_arch = "wasm32")]
async fn scores_summary_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let query = parse_query_params(&req)?;
    let score_req = match parse_score_request(&query) {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 400),
    };
    if !score_req.expanded {
        let mut resp = Response::empty()?;
        resp.set_status(204);
        return Ok(resp);
    }
    let storage = ServerlessStorage::from_env(
        &ctx.env,
        ServerlessStorage::KV_BINDING,
        ServerlessStorage::R2_BINDING,
    )
    .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let context = match load_score_context(&storage, score_req.event_id).await {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 500),
    };
    let summary = group_by_bettor_name_and_round(&context.data.score_struct);
    let markup = render_summary_scores(&summary);
    respond_html(markup.into_string())
}

#[cfg(target_arch = "wasm32")]
async fn scores_chart_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let query = parse_query_params(&req)?;
    let score_req = match parse_score_request(&query) {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 400),
    };
    let storage = ServerlessStorage::from_env(
        &ctx.env,
        ServerlessStorage::KV_BINDING,
        ServerlessStorage::R2_BINDING,
    )
    .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let context = match load_score_context(&storage, score_req.event_id).await {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 500),
    };
    let summary_scores_x = group_by_bettor_name_and_round(&context.data.score_struct);
    let detailed_scores = group_by_bettor_golfer_round(&context.data.score_struct);
    let markup = render_drop_down_bar_pure(
        &summary_scores_x,
        &detailed_scores,
        context.global_step_factor,
        &context.player_step_factors,
    );
    respond_html(markup.into_string())
}

#[cfg(target_arch = "wasm32")]
async fn scores_linescore_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let query = parse_query_params(&req)?;
    let score_req = match parse_score_request(&query) {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 400),
    };
    let storage = ServerlessStorage::from_env(
        &ctx.env,
        ServerlessStorage::KV_BINDING,
        ServerlessStorage::R2_BINDING,
    )
    .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let context = match load_score_context(&storage, score_req.event_id).await {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 500),
    };
    let bettor_struct = scores_and_last_refresh_to_line_score_tables(&context.from_db_scores);
    let refresh_data = RefreshData {
        last_refresh: context.data.last_refresh.clone(),
        last_refresh_source: context.data.last_refresh_source.clone(),
    };
    let markup = render_line_score_tables(&bettor_struct, &refresh_data);
    respond_html(markup.into_string())
}

#[cfg(target_arch = "wasm32")]
#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    let router = Router::new();

    router
        .get("/health", |_, ctx| async move {
            let _storage = ServerlessStorage::from_env(
                &ctx.env,
                ServerlessStorage::KV_BINDING,
                ServerlessStorage::R2_BINDING,
            )
            .map_err(|e| worker::Error::RustError(e.to_string()))?;
            Response::ok("ok")
        })
        .get("/scores", |req, ctx| async move { scores_handler(req, ctx).await })
        .get("/scores/summary", |req, ctx| async move {
            scores_summary_handler(req, ctx).await
        })
        .get("/scores/chart", |req, ctx| async move {
            scores_chart_handler(req, ctx).await
        })
        .get("/scores/linescore", |req, ctx| async move {
            scores_linescore_handler(req, ctx).await
        })
        .run(req, env)
        .await
}
