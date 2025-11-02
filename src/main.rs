mod rmq;
mod model;
mod config;
mod api;
mod output;
mod webhook;
mod utils;

use std::{collections::HashSet, env, process::exit};

use lapin::options::BasicAckOptions;
use futures_util::StreamExt;
use log::{STATIC_MAX_LEVEL, error};
use simplelog::{ColorChoice, TermLogger, TerminalMode};

use crate::{config::Config, model::Event, rmq::{create_akari_consumer, open_rmq_connection}};
use crate::utils::canonicalize_name;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const CONFIG_PATH: &'static str = "config/bubble.toml";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = simplelog::ConfigBuilder::new();
    builder.add_filter_ignore_str("serenity");

    TermLogger::init(
        STATIC_MAX_LEVEL, builder.build(), TerminalMode::Stderr, ColorChoice::Auto
    )?;

    let (api_user_agent, web_user_agent) = read_user_agent();

    let client = match api::Client::new(api_user_agent) {
        Ok(v) => v,
        Err(err) => {
            error!("Failed to initialize API client: {}", err);
            exit(1);
        }
    };

    let config = match config::parse_config(CONFIG_PATH) {
        Ok(v) => v,
        Err(err) => {
            error!("Failed to read config file: {}", err);
            exit(1);
        }
    };

    let conn = open_rmq_connection(&config).await?;
    let channel = conn.create_channel().await?;
    let mut consumer = create_akari_consumer(&config, &channel).await?;

    let mut wa_nations: HashSet<String> = HashSet::new();
    api::query_wa_nations(&client, &mut wa_nations).await?;

    while let Some(delivery) = consumer.next().await {
        let delivery = match delivery {
            Ok(v) => v,
            Err(err) => {
                error!("error in consumer: {}", err);
                continue;
            }
        };

        delivery
            .ack(BasicAckOptions::default())
            .await?;

        let event: Event = match str::from_utf8(&delivery.data).ok().and_then(
            |v| serde_json::from_str(v).ok()
        ) {
            Some(v) => v,
            None => continue, // invalid event! skip it
        };

        dispatch_event(event, &config, &mut wa_nations, &client, &web_user_agent).await?;
    }

    Ok(())
}

async fn dispatch_event(
    event: Event, config: &Config, wa_nations: &mut HashSet<String>, 
    client: &api::Client, web_user_agent: &String
) -> Result<(), Box<dyn std::error::Error>> {
    if event.category == "connmiss" {
        api::query_wa_nations(&client, wa_nations).await?;
        return Ok(());
    }

    if let Some(region) = &event.origin {
        if event.category.as_str() == "rmbpost" {
            let category = "rmb";
            if let Some(output_config) = config.get_event(region, category) {
                output::output_event(category, &output_config, &event, &web_user_agent).await.ok();
            }

            return Ok(());
        }

        let category = match event.category.as_str() {
            "rupdate" => "update",
            "rfeature" => "feature",
            "ndel" => "delegate",
            "rdel" => "delegate",
            "ldel" => "delegate",
            "nfound" => "found",
            "nrefound" => "found",
            "wapply" => "apply",
            "move" => {
                match &event.actor {
                    Some(nation) => if wa_nations.contains(nation) { "waleave" } else { "leave" },
                    None => return Ok(())
                }
            },
            "ncte" => {
                match &event.receptor {
                    Some(nation) => if wa_nations.contains(nation) { "wacte" } else { "cte" },
                    None => return Ok(())
                }
            },
            "wadmit" => {
                if let Some(nation) = &event.actor {
                    wa_nations.insert(nation.clone());
                    "admit"
                } else { return Ok(()); }
            },
            "wresign" => {
                if let Some(nation) = &event.actor {
                    wa_nations.remove(nation);
                    "resign"
                } else { return Ok(()); }
            },
            "wkick" => {
                if let Some(nation) = &event.receptor {
                    wa_nations.remove(nation);
                    "wakick"
                } else { return Ok(()); }
            },
            _ => {
                return Ok(());
            },
        };

        if let Some(output_config) = config.get_event(region, category) {
            output::output_event(category, &output_config, &event, &web_user_agent).await.ok();
        }
    }

    if let Some(region) = &event.destination && event.category.as_str() == "move" {
        let category = {
            match &event.actor {
                Some(nation) => if wa_nations.contains(nation) { "wajoin" } else { "join" },
                None => return Ok(())
            }
        };
        if let Some(output_config) = config.get_event(region, category) {
            output::output_event(category, &output_config, &event, &web_user_agent).await.ok();
        }
    }

    Ok(())
}

fn read_user_agent() -> (String, String) {
    let user = match env::var("NS_USER_AGENT") {
        Ok(user) => user,
        Err(err) => match err {
            env::VarError::NotPresent => {
                error!("No user agent provided, please set the NS_USER_AGENT environment variable to your main nation name");
                exit(1);
            },
            env::VarError::NotUnicode(_) => {
                error!("User agent is not valid unicode");
                exit(1);
            }
        }
    };

    let api_user_agent = format!("bubble/{} by Merethin, in use by {}", VERSION, user);
    let web_user_agent = format!("bubble__by_merethin__usedBy_{}", canonicalize_name(&user));

    (api_user_agent, web_user_agent)
}