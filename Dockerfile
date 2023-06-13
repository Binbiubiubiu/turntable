# Cargo Build

FROM rustlang/rust:nightly as builder
RUN apt-get update && apt-get install -y musl-tools
WORKDIR /app
COPY . .
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target=x86_64-unknown-linux-musl

# Final Stage

FROM alpine:latest
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/turntable /usr/local/bin/turntable
CMD ["turntable"]
