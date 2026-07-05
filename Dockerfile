# Build stage
FROM rust:1.82-alpine AS builder
WORKDIR /app

RUN apk add --no-cache musl-dev

COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy real source và force rebuild
COPY src ./src
RUN touch src/main.rs && cargo build --release

# Production stage
FROM alpine:3.19 AS runner
WORKDIR /app

COPY --from=builder /app/target/release/caro-server ./caro-server

ENV RUST_LOG=info

EXPOSE 3001

CMD ["./caro-server"]