From rust:1-slim-trixie AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# ---- runtime stage ----
FROM debian:trixie-slim
COPY --from=builder /app/target/release/rust-api /usr/local/bin/rust-api
ENV PORT=3000
EXPOSE 3000
CMD ["rust-api"]