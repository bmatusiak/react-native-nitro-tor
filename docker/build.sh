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

# Default Android API level used by this build
: "${ANDROID_API_LEVEL:=21}"
export ANDROID_API_LEVEL

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

echo "Building JS (tsdown) + Android prebuilts (skip iOS)"

# Craby's `build` command builds *all* platforms (including iOS). In Docker/Linux
# we only need Android artifacts for the example app build, so we build Android
# staticlibs directly with Cargo and place them where the generated CMake expects.

# Build TypeScript bundle/types
yarn run -T tsdown

build_android_staticlib() {
	local rust_target="$1"
	local android_abi="$2"
	local api_level="${ANDROID_API_LEVEL}"
	local cc_cmd

	case "${rust_target}" in
		aarch64-linux-android)
			cc_cmd="aarch64-linux-android${api_level}-clang"
			export CC_aarch64_linux_android="${cc_cmd}"
			export CXX_aarch64_linux_android="${cc_cmd}"
			;;
		armv7-linux-androideabi)
			cc_cmd="armv7a-linux-androideabi${api_level}-clang"
			export CC_armv7_linux_androideabi="${cc_cmd}"
			export CXX_armv7_linux_androideabi="${cc_cmd}"
			;;
		x86_64-linux-android)
			cc_cmd="x86_64-linux-android${api_level}-clang"
			export CC_x86_64_linux_android="${cc_cmd}"
			export CXX_x86_64_linux_android="${cc_cmd}"
			;;
		i686-linux-android)
			cc_cmd="i686-linux-android${api_level}-clang"
			export CC_i686_linux_android="${cc_cmd}"
			export CXX_i686_linux_android="${cc_cmd}"
			;;
		*)
			echo "ERROR Unsupported Android Rust target: ${rust_target}" >&2
			exit 1
			;;
	esac

	# Tools that autotools/libtool will try to use.
	export CC="${cc_cmd}"
	export CXX="${cc_cmd}"
	export AR="llvm-ar"
	export RANLIB="llvm-ranlib"
	export STRIP="llvm-strip"
	export NM="llvm-nm"

	echo "Building Rust staticlib for ${rust_target} (${android_abi})"
	cargo build -p react_native_nitro_tor --target "${rust_target}" --release

	local src="target/${rust_target}/release/libreactnativenitrotor.a"
	local dest_dir="android/src/main/jni/libs/${android_abi}"
	local dest="${dest_dir}/libreactnativenitrotor-prebuilt.a"

	if [[ ! -f "${src}" ]]; then
		echo "ERROR Expected build artifact not found: ${src}" >&2
		exit 1
	fi

	mkdir -p "${dest_dir}"
	cp -f "${src}" "${dest}"
}

# Ensure Cargo uses the NDK clang for each target
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="aarch64-linux-android21-clang"
export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="armv7a-linux-androideabi21-clang"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="x86_64-linux-android21-clang"
export CARGO_TARGET_I686_LINUX_ANDROID_LINKER="i686-linux-android21-clang"

build_android_staticlib aarch64-linux-android arm64-v8a
build_android_staticlib armv7-linux-androideabi armeabi-v7a
build_android_staticlib x86_64-linux-android x86_64
build_android_staticlib i686-linux-android x86

# Run the Android example build
yarn example build:android

echo "Build finished"
