#!/usr/bin/env bash
set -euo pipefail

docker build -f "$(pwd)"/docker/Dockerfile -t rn-tor-android-builder:ci .

docker run --rm -v "$(pwd)":/workspace -w /workspace rn-tor-android-builder:ci /workspace/docker/build.sh

