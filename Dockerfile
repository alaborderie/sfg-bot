# syntax=docker/dockerfile:1
FROM lukemathwalker/cargo-chef:latest-rust-1.93-alpine AS chef

RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
ENV SQLX_OFFLINE=true
RUN cargo chef cook --release --recipe-path recipe.json

COPY src ./src
COPY migrations ./migrations
RUN cargo build --release --locked

FROM alpine:3.23 AS runtime

RUN apk --no-cache add ca-certificates

WORKDIR /app

COPY --from=builder /app/target/release/sfg-bot .
COPY ANALYSIS_PROMPT.md .

CMD ["./sfg-bot"]
