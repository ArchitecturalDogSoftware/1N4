# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Copyright © 2025 RemasteredArch
#
# This file is part of 1N4.
#
# 1N4 is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public
# License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later
# version.
#
# 1N4 is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License along with 1N4. If not, see
# <https://www.gnu.org/licenses/>.

# The binary crate's name, as it appears in the `Cargo.toml`.
ARG APP_NAME=ina

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
ARG APP_NAME

SHELL ["/bin/bash", "-euo", "pipefail", "-c"]

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
    <<EOF
if [ "$BUILDPLATFORM" = "$TARGETPLATFORM" ]; then
    # Will probably only return something like `x86_64-unknown-linux-gnu`.
    native_target_triple="$(rustup target list --installed --quiet)"
    TARGET_ARCH_RUST="${native_target_triple%-unknown-linux-gnu}"
fi

dpkg --add-architecture "$TARGETARCH"
apt-get update
apt-get install --no-install-recommends --yes "musl-dev:$TARGETARCH"

# The target triple as expected by Rust.
target_triple="$TARGET_ARCH_RUST-unknown-linux-musl"
# The target triple as is used in lowercase environment variable names.
target_triple_variable="$(echo "$target_triple" | tr '-' '_')"
# The target triple as is used in uppercase environment variable names.
target_triple_variable_upper="$(echo "$target_triple_variable" | tr '[:lower:]' '[:upper:]')"
# The toolchain-specific prefix for GCC binary names.
toolchain_prefix="$TARGET_ARCH_RUST-linux-musl"

rustup target add "$target_triple"

# Make a statically-linked binary.
export "CARGO_TARGET_${target_triple_variable_upper}_RUSTFLAGS=-C link-self-contained=yes -C linker=rust-lld -C target-feature=+crt-static"
# Use the musl-specific build tools.
export "CARGO_TARGET_${target_triple_variable_upper}_LINKER=${toolchain_prefix}-gcc"
export "CC_${target_triple_variable}=${toolchain_prefix}-gcc"
export "CXX_${target_triple_variable}=${toolchain_prefix}-g++"

cargo build --target "$target_triple" --locked --profile release-super-optimized
binary_path="target/$target_triple/release-super-optimized/$APP_NAME"

mkdir /out
cp -r res /out/res 
cp "$binary_path" /out/server
EOF

# Create an output image for the target platform.
FROM alpine:3 AS server

ARG APP_NAME
ARG UID=10001
ARG TARGETARCH

SHELL ["/bin/ash", "-euo", "pipefail", "-c"]

RUN : # If this fails with "exec format error," it's probably because you don't have emulation set up.

WORKDIR /app

COPY --from=build /out/server /bin/server
RUN ln -s /bin/server "/bin/$APP_NAME"

COPY --from=build /out/res /app/res

# Create a dummy `.env` file so that `dotenvy` won't be mad it can't find one. This lets Docker handle loading the
# `.env` file without actually inserting it into container, though that's of course still an option with
# `--mount 'type=bind,source=./.env,target=/app/.env,readonly'`.
RUN touch /app/.env

RUN <<EOF
cat << EOS > /startup.sh
#!/bin/sh
until [ -w /app/log ] && [ -w /app/res/data ]; do
    sleep 1
done
/bin/$APP_NAME "\$@"
EOS

chmod +x /startup.sh
EOF

RUN adduser \
    --disabled-password \
    --gecos '' \
    --home '/nonexistent' \
    --shell '/sbin/nologin' \
    --no-create-home \
    --uid "$UID" \
    appuser
USER appuser

ENTRYPOINT ["/startup.sh"]
