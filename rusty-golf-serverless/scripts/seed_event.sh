#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: seed_event.sh EVENT_ID EVENT_DETAILS_JSON GOLFERS_JSON PLAYER_FACTORS_JSON SCORES_JSON ESPN_CACHE_JSON [LAST_REFRESH_JSON]

Seeds KV + R2 for a single event using wrangler.

Environment overrides:
  CONFIG_PATH        Path to wrangler.toml (default: rusty-golf-serverless/wrangler.toml)
  WRANGLER_ENV       Wrangler env (default: dev)
  KV_BINDING         KV binding name (default: djf_rusty_golf_kv)
  R2_BUCKET          R2 bucket name (default: djf-rusty-golf-dev for dev, djf-rusty-golf for prod)
  WRANGLER_FLAGS     Extra flags (default: --config <CONFIG_PATH>)
  WRANGLER_KV_FLAGS  Overrides WRANGLER_FLAGS for KV commands
  WRANGLER_R2_FLAGS  Overrides WRANGLER_FLAGS for R2 commands
  WRANGLER_LOG_DIR   Wrangler log directory
  XDG_CONFIG_HOME    Wrangler config directory
EOF
}

if [[ $# -lt 6 ]]; then
  usage >&2
  exit 1
fi

EVENT_ID="$1"
EVENT_DETAILS_JSON="$2"
GOLFERS_JSON="$3"
PLAYER_FACTORS_JSON="$4"
SCORES_JSON="$5"
ESPN_CACHE_JSON="$6"
LAST_REFRESH_JSON="${7:-}"

CONFIG_PATH="${CONFIG_PATH:-rusty-golf-serverless/wrangler.toml}"
WRANGLER_ENV="${WRANGLER_ENV:-dev}"
KV_BINDING="${KV_BINDING:-djf_rusty_golf_kv}"
R2_BUCKET="${R2_BUCKET:-}"
WRANGLER_FLAGS="${WRANGLER_FLAGS:---config ${CONFIG_PATH}}"
WRANGLER_KV_FLAGS="${WRANGLER_KV_FLAGS:-${WRANGLER_FLAGS} --env ${WRANGLER_ENV}}"
WRANGLER_R2_FLAGS="${WRANGLER_R2_FLAGS:-${WRANGLER_FLAGS} --env ${WRANGLER_ENV}}"
log_dir="${WRANGLER_LOG_DIR:-rusty-golf-serverless/.wrangler-logs}"
config_dir="${XDG_CONFIG_HOME:-rusty-golf-serverless/.wrangler-config}"

if [[ -z "${R2_BUCKET}" ]]; then
  if [[ "${WRANGLER_ENV}" == "prod" ]]; then
    R2_BUCKET="djf-rusty-golf"
  else
    R2_BUCKET="djf-rusty-golf-dev"
  fi
fi

for path in "${EVENT_DETAILS_JSON}" "${GOLFERS_JSON}" "${PLAYER_FACTORS_JSON}" \
  "${SCORES_JSON}" "${ESPN_CACHE_JSON}"; do
  if [[ ! -f "${path}" ]]; then
    echo "Missing file: ${path}" >&2
    exit 1
  fi
done

if [[ -n "${LAST_REFRESH_JSON}" && ! -f "${LAST_REFRESH_JSON}" ]]; then
  echo "Missing file: ${LAST_REFRESH_JSON}" >&2
  exit 1
fi

mkdir -p "${log_dir}" "${config_dir}"
export WRANGLER_LOG_DIR="${log_dir}"
export XDG_CONFIG_HOME="${config_dir}"

tmp_dir=""
cleanup() {
  if [[ -n "${tmp_dir}" ]]; then
    rm -rf "${tmp_dir}"
  fi
}
trap cleanup EXIT

if [[ -z "${LAST_REFRESH_JSON}" ]]; then
  if ! command -v jq >/dev/null 2>&1; then
    echo "jq is required to derive LAST_REFRESH_JSON; install jq or pass LAST_REFRESH_JSON." >&2
    exit 1
  fi
  tmp_dir="$(mktemp -d)"
  LAST_REFRESH_JSON="${tmp_dir}/last_refresh.json"
  jq '{ts: .last_refresh, source: .last_refresh_source}' \
    "${SCORES_JSON}" >"${LAST_REFRESH_JSON}"
fi

wrangler kv key put ${WRANGLER_KV_FLAGS} --binding "${KV_BINDING}" \
  "event:${EVENT_ID}:details" --path "${EVENT_DETAILS_JSON}"
wrangler kv key put ${WRANGLER_KV_FLAGS} --binding "${KV_BINDING}" \
  "event:${EVENT_ID}:golfers" --path "${GOLFERS_JSON}"
wrangler kv key put ${WRANGLER_KV_FLAGS} --binding "${KV_BINDING}" \
  "event:${EVENT_ID}:player_factors" --path "${PLAYER_FACTORS_JSON}"
wrangler kv key put ${WRANGLER_KV_FLAGS} --binding "${KV_BINDING}" \
  "event:${EVENT_ID}:last_refresh" --path "${LAST_REFRESH_JSON}"

wrangler r2 object put ${WRANGLER_R2_FLAGS} \
  "${R2_BUCKET}/events/${EVENT_ID}/scores.json" --file "${SCORES_JSON}"
wrangler r2 object put ${WRANGLER_R2_FLAGS} \
  "${R2_BUCKET}/cache/espn/${EVENT_ID}.json" --file "${ESPN_CACHE_JSON}"

echo "Seeded KV/R2 for event ${EVENT_ID} in env ${WRANGLER_ENV}."
