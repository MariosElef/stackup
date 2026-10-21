FROM rust:slim-trixie AS builder

WORKDIR /build

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Dependency layer: cached until Cargo.toml / Cargo.lock change.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && echo '' > src/lib.rs \
    && cargo build --release --locked \
    && rm -rf src

COPY src ./src
RUN touch src/main.rs src/lib.rs \
    && cargo build --release --locked \
    && strip target/release/{{BIN}}

FROM gcr.io/distroless/cc-debian13:nonroot

COPY --from=builder /build/target/release/{{BIN}} /usr/local/bin/{{BIN}}

USER nonroot:nonroot
EXPOSE 8000

ENTRYPOINT ["/usr/local/bin/{{BIN}}"]
