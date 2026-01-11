# bubble

Extremely customizable live multi-region activity and RMB webhook for Discord

## Requirements

Bubble requires a running RabbitMQ instance, with [Akari](https://github.com/Merethin/Akari) connected to it.

It does not connect to the NS SSE feed directly, instead listening to the activity feed provided by Akari.

## Configuration

The config file (located at `config/bubble.toml`) has four sections:

#### Input
```
[input]
exchange_name = "akari_events"
```

For the input section, specify the exchange name to listen for Akari events on.

The RabbitMQ database url should be provided in the environment or .env file as `RABBITMQ_URL`.

#### Webhooks
```
[webhooks]
main = "https://discord.com/api/webhooks/<id>/<token>"
rmb = "https://discord.com/api/webhooks/<id>/<token>"
```

A list of all webhook URLs to be used as output, each assigned to a key / name, which can be an arbitrary alphanumeric sequence.

#### Roles
```
[roles]
rmb-team = "<role_id>"
endo-team = "<role_id>"
welcome-team = "<role_id>"
```

Similarly, a list of all roles that can be pinged by the webhook, each assigned to a key / name.

#### Regional configuration
```
[region.testregionia]
default-hook = "main"
default-color = "#44BB12"
rmb = { color = "#AA00BB", hook = "rmb", mentions = ["rmb-team"] }
join = { mentions = ["welcome-team"] }
wajoin = { mentions = ["welcome-team", "endo-team"] }
admit = { mentions = ["endo-team"] }
update = {}
feature = {}
delegate = {}
leave = {}
waleave = {}
found = {}
cte = {}
wacte = {}
apply = {}
resign = {}
wakick = {}
```

Each region has its own configuration block, headed by [region.REGION_NAME].

The "default-hook" variable refers to the webhook to send happenings in that region to by default.

The "default-color" variable specifies which color to use for embeds for that region by default.

Both can be overridden by individual happenings.

After that, each happening category has a key, alongside optional settings. To enable a happening category for a region, simply equal it to an object (`update = {}`). To disable it, remove it altogether.

Optional settings: `color` overrides the embed color for that specific happening, `hook` overrides the webhook to output to for that specific happening, and `mentions` specifies a list of roles to ping for that specific happening.

Happening categories are the following:
- `rmb`: New RMB post (adds "View Post" and "Quote Post" link buttons)
- `join`: Nation moves into the region
- `wajoin`: WA Nation moves into the region (adds "Endorse Nation" link button with the #endorse anchor)
- `admit`: Nation is admitted to the WA (adds "Endorse Nation" link button with the #endorse anchor)
- `update`: Region updates
- `feature`: Region is featured
- `delegate`: Region's delegate changes
- `leave`: Nation moves out of the region
- `waleave`: WA Nation moves out of the region
- `found`: Nation is founded or refounded in the region
- `cte`: Nation ceases to exist
- `wacte`: WA Nation ceases to exist
- `apply`: Nation applies to join the WA
- `resign`: Nation resigns from the WA
- `wakick`: Nation is kicked from the WA due to rule violations

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