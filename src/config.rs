use std::collections::HashMap;
use std::fs;
use std::process::exit;
use log::{error, warn};
use serenity::{all::WebhookId, utils};
use toml::Table;
use hex_color::HexColor;
use url::Url;

use crate::webhook::Webhook;

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub color: Option<HexColor>,
    pub hook: Webhook,
    pub mentions: Vec<u64>,
}

#[derive(Debug)]
pub struct EventConfig {
    pub color: Option<HexColor>,
    pub hook: Option<String>,
    pub mentions: Vec<String>,
}

#[derive(Debug)]
pub struct RegionConfig {
    pub default_hook: Option<String>,
    pub default_color: Option<HexColor>,
    pub events: HashMap<String, EventConfig>,
}

#[derive(Debug)]
pub struct InputConfig {
    pub url: String,
    pub exchange_name: String,
}

#[derive(Debug)]
pub struct Config {
    pub input: InputConfig,
    pub webhooks: HashMap<String, (WebhookId, String)>,
    pub roles: HashMap<String, u64>,
    pub regions: HashMap<String, RegionConfig>,
}

impl Config {
    pub fn get_event(&self, region: &str, event: &str) -> Option<OutputConfig> {
        let region_config = match self.regions.get(region) {
            Some(c) => c,
            None => return None,
        };

        let event_config = match region_config.events.get(event) {
            Some(c) => c,
            None => return None,
        };

        let mut webhook: Option<(WebhookId, String)> = None; 

        if let Some(hook) = &event_config.hook {
            webhook = self.webhooks.get(hook).cloned();
        } else if let Some(hook) = &region_config.default_hook {
            webhook = self.webhooks.get(hook).cloned();
        }

        let mut result = OutputConfig { 
            color: None, hook: match webhook {
                Some(v) => v,
                None => return None
            }, mentions: Vec::new()
         };

        if let Some(color) = &event_config.color {
            result.color = Some(color.clone());
        } else if let Some(color) = &region_config.default_color {
            result.color = Some(color.clone());
        }

        for mention in event_config.mentions.iter() {
            if let Some(id) = self.roles.get(mention) {
                result.mentions.push(id.clone());
            }
        }

        Some(result)
    }
}

fn parse_webhook(hook: &str) -> Option<(WebhookId, String)> {
    let url = match Url::parse(hook).ok() {
        Some(v) => v,
        None => { return None; }
    };

    let result = utils::parse_webhook(&url);

    if let Some(pair) = result {
        return Some((pair.0, pair.1.to_owned()));
    }
    
    None
}

fn parse_webhook_map(table: &Table) -> HashMap<String, (WebhookId, String)> {
    let mut result = HashMap::new();

    for (key, value) in table.iter() {
        if let toml::Value::String(url) = value {
            if let Some(webhook) = parse_webhook(url) {
                result.insert(key.clone(), webhook);
            } else {
                warn!("Couldn't parse webhook '{}'", key);
            }
        }
    }

    result
}

fn parse_role_map(table: &Table) -> HashMap<String, u64> {
    let mut result = HashMap::new();

    for (key, value) in table.iter() {
        if let toml::Value::String(v) = value {
            if let Ok(id) = v.parse::<u64>() {
                result.insert(key.clone(), id);
            }
        }
    }

    result
}

fn parse_region(table: &Table) -> RegionConfig {
    let mut result = RegionConfig { default_hook: None, default_color: None, events: HashMap::new() };

    for (key, value) in table.iter() {
        if key == "default-hook" {
            if let toml::Value::String(v) = value {
                result.default_hook = Some(v.clone());
            }
        } else if key == "default-color" {
            if let toml::Value::String(v) = value {
                result.default_color = Some(HexColor::parse_rgb(v).expect("Not a valid color string"));
            }
        } else if let toml::Value::Table(t) = value {
            if key == "default-hook" || key == "default-color" {
                warn!("Config key {} should have a string value", key);
                continue;
            }

            let mut event: EventConfig = EventConfig { color: None, hook: None, mentions: Vec::new() };

            if let Some(toml::Value::String(s)) = t.get("color") {
                event.color = Some(HexColor::parse_rgb(s).expect("Not a valid color string"));
            }

            if let Some(toml::Value::String(s)) = t.get("hook") {
                event.hook = Some(s.clone());
            }

            if let Some(toml::Value::Array(a)) = t.get("mentions") {
                for mention in a.iter() {
                    if let toml::Value::String(s) = mention {
                        event.mentions.push(s.clone());
                    }
                }
            }

            result.events.insert(key.clone(), event);
        }
    }

    result
}

fn parse_regions(table: &Table) -> HashMap<String, RegionConfig> {
    let mut result = HashMap::new();

    for (key, value) in table.iter() {
        if let toml::Value::Table(v) = value {
            result.insert(key.clone(), parse_region(v));
        }
    }

    result
}

pub fn parse_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let table: toml::Table = toml::from_str(&contents.as_str())?;

    let input: InputConfig = match table.get("input") {
        Some(toml::Value::Table(t)) => {
            let url = match t.get("url") {
                Some(toml::Value::String(s)) => s.clone(),
                _ => {
                    error!("Config is missing required 'input.url' value!");
                    exit(1);
                }
            };

            let exchange_name = match t.get("exchange_name") {
                Some(toml::Value::String(s)) => s.clone(),
                _ => {
                    error!("Config is missing required 'input.exchange_name' value!");
                    exit(1);
                }
            };

            InputConfig { url, exchange_name }
        },
        _ => {
            error!("Config is missing required 'input' section!");
            exit(1);
        }
    };

    let webhooks = match table.get("webhooks") {
        Some(toml::Value::Table(t)) => {
            parse_webhook_map(t)
        },
        _ => {
            warn!("No webhooks specified in config!");
            HashMap::new()
        }
    };

    let roles = match table.get("roles") {
        Some(toml::Value::Table(t)) => {
            parse_role_map(t)
        },
        _ => {
            HashMap::new()
        }
    };

    let regions = match table.get("region") {
        Some(toml::Value::Table(t)) => {
            parse_regions(t)
        },
        _ => {
            warn!("No regions specified in config!");
            HashMap::new()
        }
    };

    Ok(Config { input, webhooks, roles, regions })
}