mod config;
mod api;
mod output;
mod webhook;
mod utils;
mod rmb;
mod nscode;
mod worker;
mod cache;

use std::{sync::Arc, process::exit, error::Error};

use log::error;
use serenity::all::Http;
use tokio::sync::{mpsc::Sender};

use caramel::log::setup_log;
use caramel::ns::{api::Client, UserAgent};
use caramel::akari;
use caramel::types::akari::Event;

use crate::cache::NSCache;
use crate::config::Config;
use crate::worker::NSQuery;

const PROGRAM: &str = "bubble";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHOR: &str = "Merethin";
const CONFIG_PATH: &'static str = "config/bubble.toml";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    setup_log(vec!["serenity"]);

    dotenv::dotenv().ok();

    let user_agent = UserAgent::read_from_env(PROGRAM, VERSION, AUTHOR);

    let config = config::parse_config(CONFIG_PATH).unwrap_or_else(|err| {
        error!("Failed to read config file: {err}");
        exit(1);
    });

    let url = std::env::var("RABBITMQ_URL").unwrap_or_else(|err| {
        error!("Missing RABBITMQ_URL environment variable: {err}");
        exit(1);
    });

    let conn = lapin::Connection::connect(
        &url,
        lapin::ConnectionProperties::default(),
    ).await?;

    let channel = conn.create_channel().await?;
    let mut consumer = akari::create_consumer(&channel, &config.input.exchange_name, None).await?;

    let client = Arc::new(Client::new(user_agent.clone()).unwrap_or_else(|err| {
        error!("Failed to initialize API client: {err}");
        exit(1);
    }));

    let cache = NSCache::new();

    let mut rmb_tx = worker::spawn_rmb_worker(&config, client.clone());
    let mut ns_tx = worker::spawn_ns_worker(client.clone(), cache.clone());

    ns_tx.send(NSQuery::UpdateWA).await.unwrap_or_else(|err| {
        error!("Failed to trigger WA nation update: {err}");
    });

    cache.run_tag_query(&mut ns_tx, &config).await;

    let http = Http::new("");

    while let Some(event) = akari::consume(&mut consumer).await {
        process_event(&http, event, &config, cache.clone(), &user_agent, &mut rmb_tx, &mut ns_tx).await;
    }

    Ok(())
}

async fn process_event(
    http: &Http, event: Event, config: &Config, 
    cache: Arc<NSCache>,
    user_agent: &UserAgent, 
    rmb_tx: &mut Sender<crate::rmb::Post>,
    ns_tx: &mut Sender<NSQuery>
) {
    if event.category == "connmiss" {
        ns_tx.send(NSQuery::UpdateWA).await.unwrap_or_else(|err| {
            error!("Failed to trigger WA nation update: {err}");
        });

        return;
    }

    check_and_update_tag_cloud(&event, cache.clone()).await;
    let is_wa = check_and_update_wa(&event, cache.clone()).await;

    if cache.should_run_tag_query().await {
        cache.run_tag_query(ns_tx, config).await;
    }

    if let Some(region) = &event.origin
    && let Some(category) = match_origin_category(&event, is_wa) {
        if category == "rmb" && let Some(postid) = event.data.get(0).and_then(|s| s.parse().ok()) {
            rmb_tx.send((region.clone(), postid)).await.unwrap_or_else(|err| {
                error!("Failed to send RMB post {postid} (region: {region}) to worker: {err}");
            });
        } else {
            if let Some(output_config) = config.get_region_event(region, category) {
                output::output_event(http, category, &output_config, &event, &user_agent).await.unwrap_or_else(|err| {
                    error!("Failed to send event {event:?} to webhook: {err}");
                });
            }

            for output_config in config.get_tag_events(cache.clone(), region, category).await {
                output::output_event(http, category, &output_config, &event, &user_agent).await.unwrap_or_else(|err| {
                    error!("Failed to send event {event:?} to webhook: {err}");
                });
            }
        }
    }

    if let Some(region) = &event.destination && event.category.as_str() == "move" {
        let category = if is_wa { "wajoin" } else { "join" };

        if let Some(output_config) = config.get_region_event(region, category) {
            output::output_event(http, category, &output_config, &event, &user_agent).await.unwrap_or_else(|err| {
                error!("Failed to send event {event:?} to webhook: {err}");
            });
        }

        for output_config in config.get_tag_events(cache.clone(), region, category).await {
            output::output_event(http, category, &output_config, &event, &user_agent).await.unwrap_or_else(|err| {
                error!("Failed to send event {event:?} to webhook: {err}");
            });
        }
    }

    if let Some(category) = match_world_category(&event, is_wa)
    && let Some(output_config) = config.get_world_event(category) {
        output::output_event(http, category, &output_config, &event, &user_agent).await.unwrap_or_else(|err| {
            error!("Failed to send event {event:?} to webhook: {err}");
        });
    }
}

async fn check_and_update_tag_cloud(event: &Event, cache: Arc<NSCache>) {
    match event.category.as_str() {
        "rgcte" | "govabd" => {
            if let Some(region) = &event.origin {
                if let Some(tag_entry) = cache.tag_cloud.write().await.get_mut("governorless") {
                    tag_entry.insert(region.clone());
                }
            }
        },
        "fngovrem" => {
            if let Some(region) = &event.origin {
                let mut tag_cloud = cache.tag_cloud.write().await;
                if let Some(tag_entry) = tag_cloud.get_mut("governorless") {
                    tag_entry.insert(region.clone());
                }
                if let Some(tag_entry) = tag_cloud.get_mut("frontier") {
                    tag_entry.insert(region.clone());
                }
            }
        }
        "rnewgov" => {
            if let Some(region) = &event.origin {
                if let Some(tag_entry) = cache.tag_cloud.write().await.get_mut("governorless") {
                    tag_entry.remove(region);
                }
            }
        }
        "stgovadd" => {
            if let Some(region) = &event.origin {
                let mut tag_cloud = cache.tag_cloud.write().await;
                if let Some(tag_entry) = tag_cloud.get_mut("governorless") {
                    tag_entry.remove(region);
                }
                if let Some(tag_entry) = tag_cloud.get_mut("frontier") {
                    tag_entry.remove(region);
                }
            }
        },
        "addtag" => {
            if let Some(region) = &event.origin
            && let Some(tag) = event.data.get(0).map(
                |v| v.to_lowercase().replace(' ', "_")
            ) {
                if let Some(tag_entry) = cache.tag_cloud.write().await.get_mut(&tag) {
                    tag_entry.insert(region.clone());
                }
            }
        },
        "rmtag" => {
            if let Some(region) = &event.origin
            && let Some(tag) = event.data.get(0).map(
                |v| v.to_lowercase().replace(' ', "_")
            ) {
                if let Some(tag_entry) = cache.tag_cloud.write().await.get_mut(&tag) {
                    tag_entry.remove(region);
                }
            }
        },
        "rfound" => {
            cache.tick_tag_query().await;
        }
        _ => {}
    }
}

async fn check_and_update_wa(event: &Event, cache: Arc<NSCache>) -> bool {
    match event.category.as_str() {
        "ncte" => {
            let mut wa_nations = cache.wa_nations.write().await;
            event.receptor.as_ref().is_some_and(|nation| {
                wa_nations.remove(nation)
            })
        },
        "wadmit" => {
            if let Some(nation) = &event.actor {
                cache.wa_nations.write().await.insert(nation.clone());
            }
            true
        },
        "wresign" => {
            if let Some(nation) = &event.actor {
                cache.wa_nations.write().await.remove(nation);
            }
            false
        },
        "wkick" => {
            if let Some(nation) = &event.receptor {
                cache.wa_nations.write().await.remove(nation);
            }
            false
        },
        _ => {
            let wa_nations = cache.wa_nations.read().await;
            event.actor.as_ref().is_some_and(|nation| {
                wa_nations.contains(nation)
            })
        },
    }
}

fn match_origin_category(event: &Event, is_wa: bool) -> Option<&'static str> {
    Some(match event.category.as_str() {
        "rmbpost" => "rmb",
        "rupdate" => "update",
        "rfeature" => "feature",
        "ndel" | "rdel" | "ldel" => "delegate",
        "nfound" | "nrefound" => "found",
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

fn match_world_category(event: &Event, is_wa: bool) -> Option<&'static str> {
    Some(match event.category.as_str() {
        "rsfloor" => "wa-floor",
        "rssubmit" => "wa-submit",
        "rspass" => "wa-pass",
        "rsfail" => "wa-fail",
        "rdiscard" => "wa-discard",
        "rfeature" => "feature",
        "ndel" | "rdel" | "ldel" => "delegate",
        "nfound" | "nrefound" => "found",
        "wapply" => "apply",
        "wadmit" => "admit",
        "wresign" => "resign",
        "wkick" => "kick",
        "ncte" => if is_wa { "wacte" } else { "cte" },
        _ => {
            return None;
        },
    })
}