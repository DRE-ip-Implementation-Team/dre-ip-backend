# Stage 1: build
FROM rust:1.75-bookworm as build
ARG BUILD_DIR=/app
ARG BUILD_TYPE=release

# Build dependencies only (separated for caching)
RUN cargo new --bin ${BUILD_DIR}
WORKDIR ${BUILD_DIR}
COPY ./Cargo.toml ./Cargo.toml
COPY ./backend_test ./backend_test
COPY ./protocol ./protocol
RUN if [ "${BUILD_TYPE}" = "release" ]; then BUILD_ARGS="--release"; fi; \
    cargo build ${BUILD_ARGS}
RUN rm -r src

# Build app. Stripping removes 90% of executable size so is pretty helpful!
COPY ./src ./src
ARG BUILD_ARGS=
RUN if [ "${BUILD_TYPE}" = "release" ]; then BUILD_ARGS="${BUILD_ARGS} --release"; fi; \
    cargo build ${BUILD_ARGS} && \
    strip target/${BUILD_TYPE}/dreip-backend

# Stage 2: run
FROM debian:bookworm-slim
ARG BUILD_DIR=/app
ARG BUILD_TYPE=release
ARG APP_DIR=/app
ARG APP_USER=dreip

# Create user and directory
RUN groupadd ${APP_USER} \
    && useradd -m -g ${APP_USER} ${APP_USER} \
    && mkdir -p ${APP_DIR}

# Install certificates
RUN apt-get update \
    && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy config
COPY ./Rocket.toml ${APP_DIR}
COPY ./log4rs.yaml ${APP_DIR}

# Copy executable from build stage
COPY --from=build ${BUILD_DIR}/target/${BUILD_TYPE}/dreip-backend ${APP_DIR}/dreip-backend
RUN chown -R ${APP_USER}:${APP_USER} ${APP_DIR}

# Configure runtime
ENV ROCKET_ADDRESS=0.0.0.0
EXPOSE 8000
USER ${APP_USER}
WORKDIR ${APP_DIR}
CMD ["./dreip-backend"]
