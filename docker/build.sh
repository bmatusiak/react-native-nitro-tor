#!/usr/bin/env bash
set -euo pipefail

# Minimal build script — container must mount project at /workspace
cd /workspace

# Ensure Android SDK/NDK environment (use image-installed defaults if not provided)
: "${ANDROID_SDK_ROOT:=/opt/android-sdk}"
: "${ANDROID_NDK_HOME:=$ANDROID_SDK_ROOT/ndk/25.1.8937393}"
export ANDROID_SDK_ROOT ANDROID_NDK_HOME

# Add NDK toolchain to PATH so clang/ld are available
export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"

# Default linker for Rust cross-compilation (can be overridden)
export CC="aarch64-linux-android21-clang"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC"

# Ensure Rust targets for Android is installed
/root/.cargo/bin/rustup target add aarch64-linux-android || true
/root/.cargo/bin/rustup target add armv7-linux-androideabi || true
/root/.cargo/bin/rustup target add x86_64-linux-android || true
/root/.cargo/bin/rustup target add i686-linux-android || true


# Enable corepack and run the CI yarn steps
corepack enable || true

# Install dependencies exactly as CI
yarn install --immutable

# By default skip the workspace `yarn build` (it runs native builds for all
# platforms). Set `RUN_WORKSPACE_BUILD=1` to enable it in CI.
if [ "${RUN_WORKSPACE_BUILD:-0}" = "1" ]; then
	# Optional: set `CRABY_BUILD_FLAGS` to restrict platforms for craby if the
	# CLI supports it (overrideable by CI).
	: "${CRABY_BUILD_FLAGS:=}"
	echo "Running workspace build: craby build ${CRABY_BUILD_FLAGS}"
	yarn build
else
	echo "Skipping workspace build (set RUN_WORKSPACE_BUILD=1 to enable)"
fi

# Run the Android example build
yarn example build:android

echo "Build finished"
