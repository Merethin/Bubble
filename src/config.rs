use std::{collections::HashMap, sync::Arc};
use std::fs;
use std::process::exit;
use log::{error, warn};
use toml::Table;
use hex_color::HexColor;

use caramel::webhook::{Webhook, parse_webhook_from_url};

use crate::cache::NSCache;

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub color: Option<HexColor>,
    pub hook: Webhook,
    pub mentions: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct EventConfig {
    pub color: Option<HexColor>,
    pub hook: Option<String>,
    pub mentions: Vec<String>,
}

#[derive(Debug)]
pub struct RegionConfig {
    pub default_hook: Option<String>,
    pub default_color: Option<HexColor>,
    pub exclude: Vec<String>,
    pub events: HashMap<String, EventConfig>,
}

#[derive(Debug)]
pub struct InputConfig {
    pub exchange_name: String,
}

#[derive(Debug)]
pub struct Config {
    pub input: InputConfig,
    pub webhooks: HashMap<String, Webhook>,
    pub roles: HashMap<String, u64>,
    pub regions: HashMap<String, RegionConfig>,
    pub tags: HashMap<String, RegionConfig>,
    pub world: Option<RegionConfig>,
}

impl Config {
    fn get_event_impl(&self, region_config: &RegionConfig, event: &str) -> Option<OutputConfig> {
        let Some(event_config) = region_config.events.get(event) else { return None };

        let mut webhook: Option<Webhook> = None; 

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
            result.color = Some(*color);
        } else if let Some(color) = &region_config.default_color {
            result.color = Some(*color);
        }

        for mention in &event_config.mentions {
            if let Some(id) = self.roles.get(mention) {
                result.mentions.push(*id);
            }
        }

        Some(result)
    }

    pub fn get_region_event(&self, region: &str, event: &str) -> Option<OutputConfig> {
        let Some(region_config) = self.regions.get(region) else { return None };

        return self.get_event_impl(region_config, event);
    }

    pub fn get_world_event(&self, event: &str) -> Option<OutputConfig> {
        return self.get_event_impl(self.world.as_ref()?, event);
    }

    pub async fn get_tag_events(&self, cache: Arc<NSCache>, region: &str, event: &str) -> Vec<OutputConfig> {
        cache.tag_cloud.read().await.iter().filter_map(|(tag, regions)| {
            if regions.contains(region) { Some(tag.clone()) } else { None }
        }).filter_map(|tag| {
            let config = self.tags.get(&tag).unwrap();
            if config.exclude.contains(&region.to_string()) { return None; }
            self.get_event_impl(config, event)
        }).collect()
    }
}

fn parse_webhook_map(table: &Table) -> HashMap<String, Webhook> {
    let mut result = HashMap::new();

    for (key, value) in table {
        if let toml::Value::String(url) = value {
            if let Some(webhook) = parse_webhook_from_url(url) {
                result.insert(key.clone(), webhook);
            } else {
                warn!("Couldn't parse webhook '{key}'");
            }
        }
    }

    result
}

fn parse_role_map(table: &Table) -> HashMap<String, u64> {
    let mut result = HashMap::new();

    for (key, value) in table {
        if let toml::Value::String(v) = value {
            if let Ok(id) = v.parse::<u64>() {
                result.insert(key.clone(), id);
            }
        }
    }

    result
}

fn parse_region(table: &Table) -> RegionConfig {
    let mut result = RegionConfig { 
        default_hook: None, default_color: None, exclude: Vec::new(), events: HashMap::new() 
    };

    for (key, value) in table {
        if key == "default-hook" {
            if let toml::Value::String(v) = value {
                result.default_hook = Some(v.clone());
            }
        } else if key == "default-color" {
            if let toml::Value::String(v) = value {
                result.default_color = Some(HexColor::parse_rgb(v).expect("Not a valid color string"));
            }
        } else if key == "exclude" {
            if let toml::Value::Array(a) = value {
                for region in a {
                    if let toml::Value::String(s) = region {
                        result.exclude.push(s.to_lowercase().replace(' ', "_"));
                    }
                }
            }
        } else if let toml::Value::Table(t) = value {
            if key == "default-hook" || key == "default-color" {
                warn!("Config key {key} should have a string value");
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
                for mention in a {
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

    for (key, value) in table {
        if let toml::Value::Table(v) = value {
            result.insert(key.to_lowercase().replace(' ', "_"), parse_region(v));
        }
    }

    result
}

pub fn parse_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let table: toml::Table = toml::from_str(&contents.as_str())?;

    let input: InputConfig = if let Some(toml::Value::Table(t)) = table.get("input") {
        let exchange_name = if let Some(toml::Value::String(s)) = t.get("exchange_name") { s.clone() } else {
            error!("Config is missing required 'input.exchange_name' value!");
            exit(1);
        };

        InputConfig { exchange_name }
    } else {
        error!("Config is missing required 'input' section!");
        exit(1);
    };

    let webhooks = if let Some(toml::Value::Table(t)) = table.get("webhooks") {
        parse_webhook_map(t)
    } else {
        warn!("No webhooks specified in config!");
        HashMap::new()
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
            HashMap::new()
        }
    };

    let tags = match table.get("tag") {
        Some(toml::Value::Table(t)) => {
            parse_regions(t)
        },
        _ => {
            HashMap::new()
        }
    };

    let world = match table.get("world") {
        Some(toml::Value::Table(t)) => {
            Some(parse_region(t))
        },
        _ => {
            None
        }
    };

    Ok(Config { input, webhooks, roles, regions, tags, world })
}