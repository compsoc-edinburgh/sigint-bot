FROM rust:1.78 as builder

RUN USER=root cargo new --bin sigint-bot
WORKDIR ./sigint-bot
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs

ADD . ./

RUN rm ./target/release/deps/sigint_bot*
RUN cargo build --release


FROM debian:12.5-slim
ARG APP=/usr/src/app

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*

ENV TZ=Etc/UTC \
    APP_USER=appuser \
    RUST_LOG=info

RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP}

COPY --from=builder /sigint-bot/target/release/sigint-bot ${APP}/sigint-bot
COPY ./config.toml ${APP}/config.toml

RUN chown -R $APP_USER:$APP_USER ${APP}

USER $APP_USER
WORKDIR ${APP}

CMD ["./sigint-bot"]
