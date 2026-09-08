use toml::Table;
use std::{error::Error, collections::HashMap, fs};

use crate::config::{ConfigError, rule::{Rule, ParsedRule}};

use caramel::webhook::{Webhook, parse_webhook_from_url};

fn parse_webhook_map(table: &Table) -> Result<HashMap<String, Webhook>, Box<dyn Error>> {
    let mut result = HashMap::new();

    for (key, value) in table {
        if let toml::Value::String(url) = value {
            if let Some(webhook) = parse_webhook_from_url(url) {
                result.insert(key.clone(), webhook);
            } else {
                return Err(Box::new(ConfigError { message: format!("Couldn't parse webhook '{key}'") }));
            }
        }
    }

    Ok(result)
}

fn parse_id_map(table: &Table) -> HashMap<String, u64> {
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

pub(crate) struct ConfigContext {
    pub webhooks: HashMap<String, Webhook>,
    pub roles: HashMap<String, u64>,
    pub users: HashMap<String, u64>,
}

pub fn parse_config(path: &str) -> Result<Vec<ParsedRule>, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let table: toml::Table = toml::from_str(&contents.as_str())?;

    let webhooks = if let Some(toml::Value::Table(t)) = table.get("webhooks") {
        parse_webhook_map(t)?
    } else {
        return Err(Box::new(ConfigError { message: "No webhooks specified in config!".into() }));
    };

    let roles = if let Some(toml::Value::Table(t)) = table.get("roles") {
        parse_id_map(t)
    } else { HashMap::new() };

    let users = if let Some(toml::Value::Table(t)) = table.get("users") {
        parse_id_map(t)
    } else { HashMap::new() };

    let context = ConfigContext {
        webhooks, roles, users
    };

    if let Some(toml::Value::Table(t)) = table.get("rule") {
        let mut rules = Vec::new();

        for entry in t.values() {
            let raw: Rule = entry.clone().try_into()?;
            let parsed = ParsedRule::from_raw(raw, &context)?;
            rules.push(parsed);
        }

        return Ok(rules);
    } else {
        return Err(Box::new(ConfigError { message: "No rules specified in config!".into() }));
    }
}