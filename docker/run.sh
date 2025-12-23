#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

docker build -f "${REPO_ROOT}/docker/Dockerfile" -t rn-tor-android-builder:ci "${REPO_ROOT}"

# Run the build inside the container as the non-root "builder" user,
# but first clean any root-owned node_modules from previous runs.
docker run --rm \
	-u 0:0 \
	-v "${REPO_ROOT}":/workspace \
	-w /workspace \
	rn-tor-android-builder:ci \
	bash -lc "chown -R builder:builder /workspace && su builder -c /workspace/docker/build.sh"

