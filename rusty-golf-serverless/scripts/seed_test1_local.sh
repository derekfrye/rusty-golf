#!/usr/bin/env bash
set -euo pipefail

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required for this script." >&2
  exit 1
fi

if ! command -v wrangler >/dev/null 2>&1; then
  echo "wrangler is required for this script." >&2
  exit 1
fi

EVENT_ID="${EVENT_ID:-401580351}"
KV_BINDING="${KV_BINDING:-djf_rusty_golf_kv}"
R2_BINDING="${R2_BINDING:-SCORES_R2}"
R2_BUCKET="${R2_BUCKET:-djf-rusty-golf-dev}"
WRANGLER_ENV="${WRANGLER_ENV:-dev}"
WRANGLER_FLAGS="${WRANGLER_FLAGS:---local}"
WRANGLER_KV_FLAGS="${WRANGLER_KV_FLAGS:-${WRANGLER_FLAGS} --env ${WRANGLER_ENV}}"
WRANGLER_R2_FLAGS="${WRANGLER_R2_FLAGS:-${WRANGLER_FLAGS} --env ${WRANGLER_ENV}}"
FIXTURE_JSON="${FIXTURE_JSON:-rusty-golf-tests/tests/test3_espn_json_responses.json}"
log_dir="${WRANGLER_LOG_DIR:-rusty-golf-serverless/.wrangler-logs}"
config_dir="${XDG_CONFIG_HOME:-rusty-golf-serverless/.wrangler-config}"

if [[ ! -f "${FIXTURE_JSON}" ]]; then
  echo "Fixture not found: ${FIXTURE_JSON}" >&2
  exit 1
fi

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

mkdir -p "${log_dir}" "${config_dir}"
export WRANGLER_LOG_DIR="${log_dir}"
export XDG_CONFIG_HOME="${config_dir}"

event_details_json="${tmp_dir}/event_details.json"
golfers_json="${tmp_dir}/golfers.json"
player_factors_json="${tmp_dir}/player_factors.json"
last_refresh_json="${tmp_dir}/last_refresh.json"
scores_json="${tmp_dir}/scores.json"

cat >"${event_details_json}" <<EOF
{"event_name":"PGA Championship","score_view_step_factor":3.0,"refresh_from_espn":1}
EOF

jq '.score_struct | map({eup_id, espn_id, golfer_name, bettor_name, group, score_view_step_factor: null})' \
  "${FIXTURE_JSON}" >"${golfers_json}"

jq '.score_struct | map({golfer_espn_id: .espn_id, bettor_name, step_factor: 3.0}) | unique_by(.golfer_espn_id, .bettor_name)' \
  "${FIXTURE_JSON}" >"${player_factors_json}"

cat >"${last_refresh_json}" <<EOF
{"ts":"2024-05-19T00:00:00Z","source":"Espn"}
EOF

jq '{score_struct: .score_struct, last_refresh: "2024-05-19T00:00:00", last_refresh_source: "Espn"}' \
  "${FIXTURE_JSON}" >"${scores_json}"

wrangler kv key put ${WRANGLER_KV_FLAGS} --binding "${KV_BINDING}" \
  "event:${EVENT_ID}:details" --path "${event_details_json}"
wrangler kv key put ${WRANGLER_KV_FLAGS} --binding "${KV_BINDING}" \
  "event:${EVENT_ID}:golfers" --path "${golfers_json}"
wrangler kv key put ${WRANGLER_KV_FLAGS} --binding "${KV_BINDING}" \
  "event:${EVENT_ID}:player_factors" --path "${player_factors_json}"
wrangler kv key put ${WRANGLER_KV_FLAGS} --binding "${KV_BINDING}" \
  "event:${EVENT_ID}:last_refresh" --path "${last_refresh_json}"

wrangler r2 object put ${WRANGLER_R2_FLAGS} \
  "${R2_BUCKET}/events/${EVENT_ID}/scores.json" --file "${scores_json}"
wrangler r2 object put ${WRANGLER_R2_FLAGS} \
  "${R2_BUCKET}/cache/espn/${EVENT_ID}.json" --file "${FIXTURE_JSON}"

echo "Seeded KV/R2 for event ${EVENT_ID}."
