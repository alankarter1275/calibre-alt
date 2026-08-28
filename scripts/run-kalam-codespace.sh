#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENV_FILE="${HOME}/.cache/kalam-codespace/env.sh"

"${ROOT_DIR}/scripts/codespaces-desktop.sh"
# shellcheck disable=SC1090
source "${ENV_FILE}"

cd "${ROOT_DIR}"
cargo run "$@"
