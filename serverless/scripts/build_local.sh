
#!/usr/bin/env bash
set -euo pipefail

if ! command -v wrangler >/dev/null 2>&1; then
  echo "wrangler is required for this script." >&2
  exit 1
fi

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required for this script." >&2
  exit 1
fi

if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
  echo "Rust target wasm32-unknown-unknown is required. Run: rustup target add wasm32-unknown-unknown" >&2
  exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
serverless_dir="${script_dir}/.."
workspace_root="${serverless_dir}/.."
config_path="${CONFIG_PATH:-${serverless_dir}/wrangler.toml}"
log_dir="${WRANGLER_LOG_DIR:-${workspace_root}/.wrangler-logs}"
config_dir="${XDG_CONFIG_HOME:-${workspace_root}/.wrangler-config}"
wrangler_env="${WRANGLER_ENV:-dev}"

cd "${workspace_root}"
mkdir -p "${log_dir}"
mkdir -p "${config_dir}"
export WRANGLER_LOG_DIR="${log_dir}"
export XDG_CONFIG_HOME="${config_dir}"
export RUSTFLAGS='--cfg getrandom_backend="wasm_js"'
wrangler build --config "${config_path}" --env "${wrangler_env}"
