
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
config_path="${CONFIG_PATH:-${serverless_dir}/wrangler.toml}"

cd "${serverless_dir}"
wrangler build --config "${config_path}"
