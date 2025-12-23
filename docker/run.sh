#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

docker build -f "${REPO_ROOT}/docker/Dockerfile" -t rn-tor-android-builder:ci "${REPO_ROOT}"

docker run --rm -v "${REPO_ROOT}":/workspace -w /workspace rn-tor-android-builder:ci /workspace/docker/build.sh

