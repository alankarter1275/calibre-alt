#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
./scripts/codespaces-desktop.sh >/tmp/kalam-codespaces-desktop.log 2>&1 || true
