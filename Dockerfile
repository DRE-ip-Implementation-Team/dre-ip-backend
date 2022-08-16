# Stage 1: build
FROM rust:1.62-bullseye as build
ARG BUILD_DIR=/app

# Build dependencies only (separated for caching)
RUN cargo new --bin ${BUILD_DIR}
WORKDIR ${BUILD_DIR}
COPY ./Cargo.toml ./Cargo.toml
COPY ./backend_test ./backend_test
COPY ./protocol ./protocol
RUN cargo build --release
RUN rm -r src

# Build app
COPY ./src ./src
RUN cargo build --release

# Stage 2: run
FROM debian:bullseye-slim
ARG BUILD_DIR=/app
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

# Copy executable from build stage
COPY --from=build ${BUILD_DIR}/target/release/dreip-backend ${APP_DIR}/dreip-backend
RUN chown -R ${APP_USER}:${APP_USER} ${APP_DIR}

# Copy AWS template
RUN mkdir -p /home/${APP_USER}/.aws
COPY ./credentials /home/${APP_USER}/.aws/credentials
COPY ./config /home/${APP_USER}/.aws/config
RUN chown -R ${APP_USER}:${APP_USER} /home/${APP_USER}/.aws

# Copy entrypoint code
COPY ./entrypoint.sh ${APP_DIR}

# Configure runtime
ENV ROCKET_ADDRESS=0.0.0.0
EXPOSE 8000
USER ${APP_USER}
WORKDIR ${APP_DIR}
CMD bash -c "./entrypoint.sh && ./dreip-backend"
