FROM rust:alpine AS build
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
set -euo pipefail
apk add musl-dev
cargo build --locked --release
mkdir /out
cp "target/release/$APP_NAME" /out/server
cp -r res /out/res 
EOF

FROM alpine:latest AS server
ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser
USER appuser

WORKDIR /app

COPY --from=build /out/server /bin/server
COPY --from=build /out/res /app/res

CMD ["/bin/server"]
