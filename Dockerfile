
FROM rust:1.90-alpine AS build

WORKDIR /usr/src/bubble
RUN cargo init --bin .

RUN apk add libressl-dev musl-dev

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src

RUN rm ./target/release/deps/bubble*
RUN cargo build --release

FROM rust:1.90-alpine

WORKDIR /

COPY --from=build /usr/src/bubble/target/release/bubble /usr/local/bin/bubble

CMD ["bubble"]
