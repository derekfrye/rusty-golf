#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: seed_kv_from_eup.sh EUP_JSON KV_ENV [EVENT_ID]

Seeds KV entries (event details, golfers, player_factors) from a db_prefill-style eup.json.

Environment overrides:
  CONFIG_PATH        Path to wrangler.toml (default: rusty-golf-serverless/wrangler.toml)
  WRANGLER_ENV       Wrangler env (default: dev)
  KV_ENV             KV env to target (dev or prod)
  REFRESH_FROM_ESPN  refresh_from_espn value (default: 1)
  WRANGLER_FLAGS     Extra flags (default: --config <CONFIG_PATH>)
  WRANGLER_KV_FLAGS  Overrides WRANGLER_FLAGS for KV commands
  WRANGLER_LOG_DIR   Wrangler log directory
  XDG_CONFIG_HOME    Wrangler config directory
EOF
}

if [[ $# -lt 2 ]]; then
  usage >&2
  exit 1
fi

EUP_JSON="$1"
KV_ENV="$2"
EVENT_ID="${3:-}"

CONFIG_PATH="${CONFIG_PATH:-rusty-golf-serverless/wrangler.toml}"
WRANGLER_ENV="${WRANGLER_ENV:-dev}"
REFRESH_FROM_ESPN="${REFRESH_FROM_ESPN:-1}"
WRANGLER_FLAGS="${WRANGLER_FLAGS:---config ${CONFIG_PATH} --remote --preview false}"
WRANGLER_KV_FLAGS="${WRANGLER_KV_FLAGS:-${WRANGLER_FLAGS} --env ${WRANGLER_ENV}}"
log_dir="${WRANGLER_LOG_DIR:-rusty-golf-serverless/.wrangler-logs}"
config_dir="${XDG_CONFIG_HOME:-rusty-golf-serverless/.wrangler-config}"

if [[ ! -f "${EUP_JSON}" ]]; then
  echo "Missing file: ${EUP_JSON}" >&2
  exit 1
fi

if [[ -z "${KV_ENV}" ]]; then
  echo "Missing KV env." >&2
  usage >&2
  exit 1
fi

if [[ "${KV_ENV}" != "dev" && "${KV_ENV}" != "prod" ]]; then
  echo "KV env must be 'dev' or 'prod'." >&2
  exit 1
fi

mkdir -p "${log_dir}" "${config_dir}"
export WRANGLER_LOG_DIR="${log_dir}"
export XDG_CONFIG_HOME="${config_dir}"

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

python_output="$(python - "${EUP_JSON}" "${CONFIG_PATH}" "${KV_ENV}" "${tmp_dir}" "${EVENT_ID}" "${REFRESH_FROM_ESPN}" <<'PY'
import json
import os
import sys
import tomllib

eup_path, config_path, kv_env, out_dir, event_id_arg, refresh_from_espn = sys.argv[1:]
refresh_from_espn = int(refresh_from_espn)
event_id_filter = int(event_id_arg) if event_id_arg else None

with open(config_path, "rb") as handle:
    config = tomllib.load(handle)

env_config = config.get("env", {}).get(kv_env, {})
kv_namespaces = env_config.get("kv_namespaces", [])
if not kv_namespaces:
    raise SystemExit(f"No kv_namespaces found for env '{kv_env}' in {config_path}")

kv_namespace_id = kv_namespaces[0].get("id")
if not kv_namespace_id:
    raise SystemExit(f"Missing kv_namespaces[0].id for env '{kv_env}' in {config_path}")

with open(eup_path, "r", encoding="utf-8") as handle:
    data = json.load(handle)

events = [e for e in data if event_id_filter is None or e["event"] == event_id_filter]
if not events:
    msg = f"No events found for event_id={event_id_filter}" if event_id_filter else "No events found"
    raise SystemExit(msg)

for event in events:
    event_id = event["event"]
    event_dir = os.path.join(out_dir, str(event_id))
    os.makedirs(event_dir, exist_ok=True)

    details = {
        "event_name": event["name"],
        "score_view_step_factor": event["score_view_step_factor"],
        "refresh_from_espn": refresh_from_espn,
    }
    with open(os.path.join(event_dir, "event_details.json"), "w", encoding="utf-8") as out:
        json.dump(details, out, separators=(",", ":"))

    data_to_fill = event["data_to_fill_if_event_and_year_missing"][0]
    golfers_by_id = {g["espn_id"]: g["name"] for g in data_to_fill["golfers"]}
    bettor_counts = {}
    golfers_out = []
    eup_id = 1
    for entry in data_to_fill["event_user_player"]:
        bettor = entry["bettor"]
        bettor_counts[bettor] = bettor_counts.get(bettor, 0) + 1
        golfer_name = golfers_by_id.get(entry["golfer_espn_id"])
        if golfer_name is None:
            raise SystemExit(
                f"Missing golfer_espn_id {entry['golfer_espn_id']} in golfers list for event {event_id}"
            )

        golfers_out.append({
            "eup_id": eup_id,
            "espn_id": entry["golfer_espn_id"],
            "golfer_name": golfer_name,
            "bettor_name": bettor,
            "group": bettor_counts[bettor],
            "score_view_step_factor": entry.get("score_view_step_factor"),
        })
        eup_id += 1

    with open(os.path.join(event_dir, "golfers.json"), "w", encoding="utf-8") as out:
        json.dump(golfers_out, out, separators=(",", ":"))

    player_factors = [
        {
            "golfer_espn_id": entry["golfer_espn_id"],
            "bettor_name": entry["bettor"],
            "step_factor": entry["score_view_step_factor"],
        }
        for entry in data_to_fill["event_user_player"]
        if "score_view_step_factor" in entry
    ]
    with open(os.path.join(event_dir, "player_factors.json"), "w", encoding="utf-8") as out:
        json.dump(player_factors, out, separators=(",", ":"))

event_ids = "\n".join(str(event["event"]) for event in events)
print(kv_namespace_id)
print(event_ids)
PY
)"

KV_NAMESPACE_ID="$(printf '%s\n' "${python_output}" | head -n 1)"
event_ids="$(printf '%s\n' "${python_output}" | tail -n +2)"

if [[ -z "${KV_NAMESPACE_ID}" ]]; then
  echo "Missing KV namespace id in ${CONFIG_PATH} for env ${KV_ENV}." >&2
  exit 1
fi

if [[ -z "${event_ids}" ]]; then
  echo "No events found to seed." >&2
  exit 1
fi

while IFS= read -r event_id; do
  event_dir="${tmp_dir}/${event_id}"
  wrangler kv key put ${WRANGLER_KV_FLAGS} --namespace-id "${KV_NAMESPACE_ID}" \
    "event:${event_id}:details" --path "${event_dir}/event_details.json"
  wrangler kv key put ${WRANGLER_KV_FLAGS} --namespace-id "${KV_NAMESPACE_ID}" \
    "event:${event_id}:golfers" --path "${event_dir}/golfers.json"
  wrangler kv key put ${WRANGLER_KV_FLAGS} --namespace-id "${KV_NAMESPACE_ID}" \
    "event:${event_id}:player_factors" --path "${event_dir}/player_factors.json"
  echo "Seeded KV for event ${event_id}."
done <<< "${event_ids}"

echo "KV seed complete for env ${WRANGLER_ENV}."
