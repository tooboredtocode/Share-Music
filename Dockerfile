FROM rust:latest as build

# create a new empty shell project
RUN USER=root cargo new --bin share-music
WORKDIR /share-music

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release & rm src/*.rs

# copy your source tree
COPY ./src ./src
COPY ./build.rs ./build.rs

# build for release
RUN cargo build --release

FROM gcr.io/distroless/java17 as libz-required

# our final base
FROM gcr.io/distroless/cc

# copy the build artifact from the build stage
COPY --from=libz-required --chown=root:root /lib/x86_64-linux-gnu/libz.so.1 /lib/x86_64-linux-gnu/libz.so.1
COPY --from=build /share-music/target/release/share-music /

# set the startup command to run your binary
CMD ["./share-music"]