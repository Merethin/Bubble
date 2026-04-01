mod config;
mod api;
mod output;
mod webhook;
mod utils;
mod rmb;
mod nscode;
mod worker;
mod cache;
mod events;

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
use crate::events::{check_and_update_wa, match_origin_category, match_world_category};

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

    let mut ns_tx = worker::spawn_ns_worker(client.clone(), cache.clone());

    ns_tx.send(NSQuery::UpdateWA).await.unwrap_or_else(|err| {
        error!("Failed to trigger WA nation update: {err}");
    });

    cache.run_tag_query(&mut ns_tx, &config).await;

    let http = Http::new("");

    while let Some(event) = akari::consume(&mut consumer).await {
        process_event(&http, event, &config, cache.clone(), &user_agent, &mut ns_tx).await;
    }

    Ok(())
}

async fn process_event(
    http: &Http, event: Event, config: &Config, 
    cache: Arc<NSCache>,
    user_agent: &UserAgent, 
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