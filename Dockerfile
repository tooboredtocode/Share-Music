FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /share-music

FROM chef AS planner
# prepare dependencies for caching
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# build project dependencies
COPY --from=planner /share-music/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
# build project
COPY . .
RUN cargo build --release --bin share-music

FROM gcr.io/distroless/cc-debian13:latest AS runtime

COPY --from=builder /share-music/target/release/share-music /share-music
ENTRYPOINT ["./share-music"]
