
FROM rust:1.90-alpine AS build

WORKDIR /usr/src/bubble
RUN cargo init --bin .

RUN mkdir -p ./caramel
RUN cargo init --bin caramel

RUN apk add libressl-dev musl-dev

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./caramel/Cargo.lock ./caramel/Cargo.lock
COPY ./caramel/Cargo.toml ./caramel/Cargo.toml

RUN cargo build --release
RUN rm src/*.rs
RUN rm caramel/src/*.rs

COPY ./src ./src
COPY ./caramel/src ./caramel/src

RUN rm ./target/release/deps/bubble*
RUN cargo build --release

FROM rust:1.90-alpine

WORKDIR /

COPY --from=build /usr/src/bubble/target/release/bubble /usr/local/bin/bubble

CMD ["bubble"]
