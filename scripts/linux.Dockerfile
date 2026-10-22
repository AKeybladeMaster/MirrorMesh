# Build toolchain only. No tests, app startup, signing credentials or release upload.
FROM ubuntu:24.04@sha256:68ee22004a9109b44973313b02ff7b1de250f1de0f5ecb628d73f579aaff33ac
ENV DEBIAN_FRONTEND=noninteractive CI=true FLUTTER_SUPPRESS_ANALYTICS=true \
    FLUTTER_ROOT=/opt/flutter PUB_CACHE=/opt/pub-cache \
    CARGO_HOME=/opt/cargo RUSTUP_HOME=/opt/rustup RUSTUP_TOOLCHAIN=1.99.1
ENV PATH=/opt/flutter/bin:/opt/cargo/bin:$PATH
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates clang cmake ninja-build pkg-config libgtk-3-dev liblzma-dev \
    libstdc++-12-dev libsecret-1-dev libssl-dev git curl unzip xz-utils python3 file \
    && apt-get clean
RUN git clone --depth 1 --branch 3.47.7 https://github.com/flutter/flutter.git /opt/flutter \
    && test "$(git -C /opt/flutter rev-parse HEAD)" = abaf9c523780a608bd46686fd5e53740a07077f8 \
    && flutter precache --linux
RUN curl --fail --location --proto '=https' --tlsv1.2 \
    https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init -o /tmp/rustup-init \
    && chmod +x /tmp/rustup-init \
    && /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain 1.99.1 \
       --component rustfmt --component clippy
WORKDIR /work
