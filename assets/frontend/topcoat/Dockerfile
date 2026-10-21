FROM rust:slim-trixie AS builder

WORKDIR /build

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config \
    && rm -rf /var/lib/apt/lists/*

# The release binary loads its assets (stylesheet, browser runtime) from a folder next to
# itself, and only the Topcoat CLI can produce that folder. Pinned to the revision this
# project was generated with.
RUN cargo install --git https://github.com/tokio-rs/topcoat --rev {{REV}} topcoat-cli --locked

# No dependency-only layer, unlike the backend: build.rs needs the real sources to scan for
# Tailwind classes.
COPY . .
RUN topcoat asset bundle --release --bin {{BIN}} \
    && strip target/release/{{BIN}}

FROM gcr.io/distroless/cc-debian13:nonroot

COPY --from=builder /build/target/release/{{BIN}} /app/{{BIN}}
COPY --from=builder /build/target/release/assets /app/assets

ENV HOST=0.0.0.0 \
    PORT=3000

USER nonroot:nonroot
EXPOSE 3000

ENTRYPOINT ["/app/{{BIN}}"]
