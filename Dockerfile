FROM rust:1.81-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/j /usr/local/bin/j
CMD ["j", "--help"]
