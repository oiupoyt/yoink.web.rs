# Multi-stage build
FROM rust:1.80-slim-bookworm AS builder
WORKDIR /usr/src/yoink
COPY . .
RUN cargo build --release --workspace

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y python3 yt-dlp ffmpeg ca-certificates && rm -rf /var/lib/apt/lists/*
RUN useradd -m yoink
USER yoink
WORKDIR /home/yoink
COPY --from=builder /usr/src/yoink/target/release/yoink /usr/local/bin/
COPY --from=builder /usr/src/yoink/target/release/yoink-web /usr/local/bin/
VOLUME /data
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s CMD curl -f http://localhost:8080/api/health || exit 1
CMD ["yoink-web"]
