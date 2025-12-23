FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive
ENV ANDROID_SDK_ROOT=/opt/android-sdk
ENV PATH=${ANDROID_SDK_ROOT}/cmdline-tools/tools/bin:${ANDROID_SDK_ROOT}/platform-tools:${PATH}

# Install base packages and OpenJDK 17
RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
		ca-certificates curl unzip zip git build-essential libevent-dev cmake ninja-build python3 python3-pip wget gnupg2 lsb-release pkg-config libssl-dev openjdk-17-jdk-headless \
		autoconf automake libtool m4 gettext \
	&& rm -rf /var/lib/apt/lists/*

# Install Node 20 and enable Corepack (for Yarn 4)
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
	&& apt-get update \
	&& apt-get install -y nodejs \
	&& corepack enable \
	&& rm -rf /var/lib/apt/lists/*

# Install Android commandline tools and NDK
RUN mkdir -p ${ANDROID_SDK_ROOT} /tmp/sdk \
	&& curl -fSL https://dl.google.com/android/repository/commandlinetools-linux-9477386_latest.zip -o /tmp/cmdline-tools.zip \
	&& unzip -q /tmp/cmdline-tools.zip -d /tmp/sdk \
	&& mkdir -p ${ANDROID_SDK_ROOT}/cmdline-tools \
	&& mv /tmp/sdk/cmdline-tools ${ANDROID_SDK_ROOT}/cmdline-tools/tools \
	&& rm -rf /tmp/cmdline-tools.zip /tmp/sdk

ENV PATH=${ANDROID_SDK_ROOT}/cmdline-tools/tools/bin:${ANDROID_SDK_ROOT}/platform-tools:${PATH}

# Accept licenses and install platform tools + NDK
RUN yes | ${ANDROID_SDK_ROOT}/cmdline-tools/tools/bin/sdkmanager --sdk_root=${ANDROID_SDK_ROOT} --licenses || true
RUN ${ANDROID_SDK_ROOT}/cmdline-tools/tools/bin/sdkmanager --sdk_root=${ANDROID_SDK_ROOT} "platform-tools" "platforms;android-33" "ndk;26.1.10909125" || true

# Create an unprivileged build user
RUN useradd -m builder

# Switch to the unprivileged user for Rust and builds
USER builder

# Install Rust toolchain as the unprivileged user
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

# Ensure Cargo (installed under the builder home) is on PATH
ENV CARGO_HOME=/home/builder/.cargo
ENV PATH=${CARGO_HOME}/bin:${ANDROID_SDK_ROOT}/cmdline-tools/tools/bin:${ANDROID_SDK_ROOT}/platform-tools:${PATH}

# Add Rust Android targets for the builder user
# RUN rustup target add aarch64-linux-android || true
# RUN rustup target add armv7-linux-androideabi || true
# RUN rustup target add x86_64-linux-android || true
# RUN rustup target add i686-linux-android || true

WORKDIR /workspace

