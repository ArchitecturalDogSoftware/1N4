# Run build step natively.
#
# This has to be on Debian because AFAIK Alpine doesn't provide foreign architecture packages to enable
# cross-compilation.
FROM --platform="$BUILDPLATFORM" rust:bookworm AS build

# The host OS and architecture from Docker. This and `TARGETPLATFORM` rely on using the BuildKit backend for Docker.
ARG BUILDPLATFORM
# The target OS and architecture from Docker.
ARG TARGETPLATFORM
# The target architecture from Docker.
ARG TARGETARCH
# The target architecture for Rust. Docker and Rust use different architecture names, so this has to be provided
# manually.
#
# E.g., for `TARGET_PLATFORM=linux/arm64`, one should match with `TARGET_ARCH_RUST=aarch64`, which will generate the
# target triple `aarch64-unknown-linux-musl` for Rust.
ARG TARGET_ARCH_RUST
# The binary crate's name, as it appears in the `Cargo.toml`.
ARG APP_NAME=ina

WORKDIR /app
RUN \
    --mount=type=bind,source=.git,target=.git,readonly \
    --mount=type=bind,source=.cargo,target=.cargo,readonly \
    --mount=type=bind,source=src,target=src,readonly \
    --mount=type=bind,source=lib,target=lib,readonly \
    --mount=type=bind,source=res,target=res,readonly \
    --mount=type=bind,source=docs,target=docs,readonly \
    --mount=type=bind,source=rust-toolchain.toml,target=rust-toolchain.toml,readonly \
    --mount=type=bind,source=build.rs,target=build.rs,readonly \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml,readonly \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock,readonly \
    << EOF
# No `-o pipefail` because Debian Bookworm's `sh` shell doesn't provide it. POSIX 2024 will fix this Soon:tm:.
set -eu

if [ "$BUILDPLATFORM" = "$TARGETPLATFORM" ]; then
    apt-get update
    apt-get install -y musl-dev

    cargo build --locked --release
    binary_path="target/release/$APP_NAME"
else
    # The target triple as expected by Rust.
    target_triple="$TARGET_ARCH_RUST-unknown-linux-musl"
    # The target triple as is used in lowercase environment variable names.
    target_triple_variable="$(echo "$target_triple" | tr '-' '_')"
    # The target triple as is used in uppercase environment variable names.
    target_triple_variable_upper="$(echo "$target_triple_variable" | tr '[:lower:]' '[:upper:]')"
    # The toolchain-specific prefix for GCC binary names.
    toolchain_prefix="$TARGET_ARCH_RUST-linux-musl"

    dpkg --add-architecture "$TARGETARCH"
    apt-get update
    apt-get install -y "musl-dev:$TARGETARCH"

    rustup target add "$target_triple"

    export "CARGO_TARGET_${target_triple_variable_upper}_LINKER=${toolchain_prefix}-gcc"
    export "CC_${target_triple_variable}=${toolchain_prefix}-gcc"
    export "CXX_${target_triple_variable}=${toolchain_prefix}-g++"

    cargo build --target "$target_triple" --locked --release
    binary_path="target/$target_triple/release/$APP_NAME"
fi

mkdir /out
cp -r res /out/res 
cp "$binary_path" /out/server
EOF

# Create an output image for the target platform.
FROM alpine:latest AS server
ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos '' \
    --home '/nonexistent' \
    --shell '/sbin/nologin' \
    --no-create-home \
    --uid "${UID}" \
    appuser
USER appuser

WORKDIR /app

COPY --from=build /out/server /bin/server
COPY --from=build /out/res /app/res

CMD ["/bin/server"]
