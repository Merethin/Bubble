mod config;
mod api;
mod output;
mod webhook;
mod utils;
mod rmb;
mod nscode;
mod worker;

use std::{collections::HashSet, sync::Arc, process::exit, error::Error};

use log::error;
use serenity::all::Http;
use tokio::sync::{RwLock, mpsc::Sender};

use caramel::log::setup_log;
use caramel::ns::{api::Client, UserAgent};
use caramel::akari;
use caramel::types::akari::Event;

use crate::config::Config;

const PROGRAM: &str = "bubble";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHOR: &str = "Merethin";
const CONFIG_PATH: &'static str = "config/bubble.toml";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    setup_log(vec!["serenity"]);

    let user_agent = UserAgent::read_from_env(PROGRAM, VERSION, AUTHOR);

    let config = config::parse_config(CONFIG_PATH).unwrap_or_else(|err| {
        error!("Failed to read config file: {}", err);
        exit(1);
    });

    let conn = lapin::Connection::connect(
        &config.input.url,
        lapin::ConnectionProperties::default(),
    ).await?;

    let channel = conn.create_channel().await?;
    let mut consumer = akari::create_consumer(&channel, &config.input.exchange_name, None).await?;

    let client = Arc::new(Client::new(user_agent.clone()).unwrap_or_else(|err| {
        error!("Failed to initialize API client: {}", err);
        exit(1);
    }));

    let wa_nations: Arc<RwLock<HashSet<String>>> = Arc::new(RwLock::new(HashSet::new()));

    let mut rmb_tx = worker::spawn_rmb_worker(&config, client.clone());
    let mut wa_tx = worker::spawn_wa_worker(client.clone(), wa_nations.clone());

    // Trigger a "fetch WA nations" API call
    wa_tx.send(()).await.unwrap_or_else(|err| {
        error!("Failed to trigger WA nation update: {err}");
    });

    let http = Http::new("");

    while let Some(event) = akari::consume(&mut consumer).await {
        process_event(&http, event, &config, wa_nations.clone(), &user_agent, &mut rmb_tx, &mut wa_tx).await;
    }

    Ok(())
}

async fn process_event(
    http: &Http, event: Event, config: &Config, 
    wa_nations: Arc<RwLock<HashSet<String>>>, 
    user_agent: &UserAgent, 
    rmb_tx: &mut Sender<crate::rmb::Post>,
    wa_tx: &mut Sender<()>
) {
    if event.category == "connmiss" {
        // Trigger a "fetch WA nations" API call
        wa_tx.send(()).await.unwrap_or_else(|err| {
            error!("Failed to trigger WA nation update: {err}");
        });

        return;
    }

    let is_wa = check_and_update_wa(&event, wa_nations).await;

    if let Some(region) = &event.origin
    && let Some(category) = match_origin_category(&event, is_wa)
    && let Some(output_config) = config.get_event(region, category) {
        if category == "rmb" && let Some(postid) = event.data.get(0).and_then(|s| s.parse().ok()) {
            rmb_tx.send((region.clone(), postid)).await.unwrap_or_else(|err| {
                error!("Failed to send RMB post {postid} (region: {region}) to worker: {err}");
            });
        } else {
            output::output_event(http, category, &output_config, &event, &user_agent).await.unwrap_or_else(|err| {
                error!("Failed to send event {event:?} to webhook: {err}");
            });
        }
    }

    if let Some(region) = &event.destination 
    && event.category.as_str() == "move"
    && let category = if is_wa { "wajoin" } else { "join" }
    && let Some(output_config) = config.get_event(region, category) {
        output::output_event(http, category, &output_config, &event, &user_agent).await.unwrap_or_else(|err| {
            error!("Failed to send event {event:?} to webhook: {err}");
        });
    }

    if let Some(category) = match_world_category(&event)
    && let Some(output_config) = config.get_world_event(category) {
        output::output_event(http, category, &output_config, &event, &user_agent).await.unwrap_or_else(|err| {
            error!("Failed to send event {event:?} to webhook: {err}");
        });
    }
}

async fn check_and_update_wa(event: &Event, wa_nations: Arc<RwLock<HashSet<String>>>) -> bool {
    match event.category.as_str() {
        "ncte" => {
            let mut wa_nations = wa_nations.write().await;
            event.receptor.as_ref().map(|nation| {
                wa_nations.remove(nation)
            }).unwrap_or(false)
        },
        "wadmit" => {
            if let Some(nation) = &event.actor {
                let mut wa_nations = wa_nations.write().await;
                wa_nations.insert(nation.clone());
            }
            true
        },
        "wresign" => {
            if let Some(nation) = &event.actor {
                let mut wa_nations = wa_nations.write().await;
                wa_nations.remove(nation);
            }
            false
        },
        "wkick" => {
            if let Some(nation) = &event.receptor {
                let mut wa_nations = wa_nations.write().await;
                wa_nations.remove(nation);
            }
            false
        },
        _ => {
            let wa_nations = wa_nations.read().await;
            event.actor.as_ref().map(|nation| {
                wa_nations.contains(nation)
            }).unwrap_or(false)
        },
    }
}

fn match_origin_category(event: &Event, is_wa: bool) -> Option<&'static str> {
    Some(match event.category.as_str() {
        "rmbpost" => "rmb",
        "rupdate" => "update",
        "rfeature" => "feature",
        "ndel" => "delegate",
        "rdel" => "delegate",
        "ldel" => "delegate",
        "nfound" => "found",
        "nrefound" => "found",
        "wapply" => "apply",
        "wadmit" => "admit",
        "wresign" => "resign",
        "wkick" => "kick",
        "move" => if is_wa { "waleave" } else { "leave" },
        "ncte" => if is_wa { "wacte" } else { "cte" },
        _ => {
            return None;
        },
    })
}

fn match_world_category(event: &Event) -> Option<&'static str> {
    Some(match event.category.as_str() {
        "rsfloor" => "wa-floor",
        "rssubmit" => "wa-submit",
        "rspass" => "wa-pass",
        "rsfail" => "wa-fail",
        "rsdiscard" => "wa-discard",
        _ => {
            return None;
        },
    })
}