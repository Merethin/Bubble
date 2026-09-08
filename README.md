# bubble

Extremely customizable live multi-region activity and RMB webhook for Discord

## Requirements

Bubble requires a running RabbitMQ instance, with [Akari](https://github.com/Merethin/Akari) connected to it.

It does not connect to the NS SSE feed directly, instead listening to the activity feed provided by Akari.

## Configuration

The config file should be located at `config/bubble.toml`. Its format is specified in [docs/config.md](docs/config.md).

The RabbitMQ database url should be provided in the environment or .env file as `RABBITMQ_URL`.

## Setup

**Make sure to use the `--recursive` flag when cloning the repository or download submodules before building!**

Run `cargo build --release` to compile the program. You'll need a recent version of Rust.

Run it with `NS_USER_AGENT=[YOUR MAIN NATION NAME] ./target/release/bubble` (with the appropriate variables in .env).

Alternatively, you can set up a Docker container.

Building it: `docker build --tag bubble .`

Running it: `docker run -e NS_USER_AGENT=[YOUR MAIN NATION NAME] -e RABBITMQ_URL=[...] bubble`

Note: to pass your config file over to Bubble, you must bind mount the directory it is in:

`docker run -e NS_USER_AGENT=[YOUR MAIN NATION NAME] -e RABBITMQ_URL=[...] -v ./config:/config bubble`

Inside Docker, Bubble looks for the config file in `/config/bubble.toml`. If it isn't behaving like you expect, make sure the file is present/mounted in some way.