FROM rust:1.88 as builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/workdesk-core/Cargo.toml crates/workdesk-core/Cargo.toml
COPY crates/workdesk-domain/Cargo.toml crates/workdesk-domain/Cargo.toml
COPY crates/workdesk-runner/Cargo.toml crates/workdesk-runner/Cargo.toml
COPY apps/workdesk-desktop/Cargo.toml apps/workdesk-desktop/Cargo.toml
RUN mkdir -p crates/workdesk-core/src crates/workdesk-domain/src crates/workdesk-runner/src apps/workdesk-desktop/src && \
    echo "fn main(){}" > crates/workdesk-core/src/main.rs && \
    echo "" > crates/workdesk-core/src/lib.rs && \
    echo "" > crates/workdesk-domain/src/lib.rs && \
    echo "" > crates/workdesk-runner/src/lib.rs && \
    echo "fn main(){}" > apps/workdesk-desktop/src/main.rs && \
    cargo build -p workdesk-core --release

COPY . .
RUN cargo build -p workdesk-core --release

FROM debian:bookworm-slim
WORKDIR /srv/workdesk
RUN useradd -m workdesk
COPY --from=builder /app/target/release/workdesk-core /usr/local/bin/workdesk-core
USER workdesk
EXPOSE 4000
CMD ["workdesk-core"]
