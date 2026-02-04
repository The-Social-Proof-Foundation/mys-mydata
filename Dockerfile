# Start with a Rust base image
FROM rust:1.88-bullseye  AS builder

ARG PROFILE=release

WORKDIR work

COPY ./crates ./crates
COPY ./Cargo.toml ./Cargo.lock ./

ARG GIT_REVISION
ENV GIT_REVISION=$GIT_REVISION

RUN cargo build --bin key-server --profile $PROFILE --config net.git-fetch-with-cli=true
FROM debian:bullseye-slim AS runtime

EXPOSE 2024

RUN apt-get update && apt-get install -y cmake clang libpq5 ca-certificates libpq-dev postgresql && \
    mkdir -p /app/config

COPY --from=builder /work/target/release/key-server /opt/key-server/bin/key-server

# Copy config file to default location (can be overridden by CONFIG_PATH)
COPY ./key-server-config-railway.yaml /app/config/key-server-config-railway.yaml

# Handle all environment variables
RUN echo '#!/bin/bash\n\
# Export all environment variables\n\
for var in $(env | cut -d= -f1); do\n\
    export "$var"\n\
done\n\
\n\
exec /opt/key-server/bin/key-server "$@"' > /opt/key-server/entrypoint.sh && \
    chmod +x /opt/key-server/entrypoint.sh

ENTRYPOINT ["/opt/key-server/entrypoint.sh"]
