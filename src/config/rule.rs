use std::error::Error;
use caramel::webhook::Webhook;
use hex_color::HexColor;
use regex::Regex;
use serde::Deserialize;

use crate::{config::{ConfigError, parser::ConfigContext}, events::EventData, output::{LINK_MAP, LinkGenerator}};

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    // Conditions
    pub if_event: Option<Vec<String>>,
    pub unless_event: Option<Vec<String>>,
    pub if_nation: Option<Vec<String>>,
    pub unless_nation: Option<Vec<String>>,
    pub if_region: Option<Vec<String>>,
    pub unless_region: Option<Vec<String>>,
    pub if_content: Option<Vec<String>>,
    pub unless_content: Option<Vec<String>>,
    pub if_wa: Option<bool>,

    // Parameters
    pub webhook: String,
    pub color: Option<String>,
    #[serde(default)]
    pub role_mentions: Vec<String>,
    #[serde(default)]
    pub user_mentions: Vec<String>,
    #[serde(default)]
    pub links: Vec<String>,
    pub content_limit: Option<usize>
}

#[derive(Debug, Clone)]
pub struct ParsedRule {
    // Conditions
    pub event: ConditionList,
    pub nation: ConditionList,
    pub region: ConditionList,
    pub content: ConditionList,
    pub wa: Option<bool>,

    // Parameters
    pub webhook: Webhook,
    pub color: HexColor,
    pub role_mentions: Vec<u64>,
    pub user_mentions: Vec<u64>,
    pub links: Vec<LinkGenerator>,
    pub content_limit: Option<usize>
}

impl ParsedRule {
    pub fn from_raw(raw: Rule, context: &ConfigContext) -> Result<Self, Box<dyn Error>> {
        let mut role_mentions = Vec::new();
        let mut user_mentions = Vec::new();
        let mut links = Vec::new();

        for role in raw.role_mentions {
            role_mentions.push(context.roles.get(&role).cloned().ok_or(ConfigError { message: format!("role '{}' does not exist", role)})?);
        }

        for user in raw.user_mentions {
            user_mentions.push(context.users.get(&user).cloned().ok_or(ConfigError { message: format!("user '{}' does not exist", user)})?);
        }

        for link in raw.links {
            links.push(LINK_MAP.get(link.as_str()).cloned().ok_or(ConfigError { message: format!("link '{}' does not exist", link)})?);
        }

        Ok(Self {
            event: ConditionList::new(raw.if_event, raw.unless_event)?,
            nation: ConditionList::new(raw.if_nation, raw.unless_nation)?,
            region: ConditionList::new(raw.if_region, raw.unless_region)?,
            content: ConditionList::new(raw.if_content, raw.unless_content)?,
            wa: raw.if_wa,

            webhook: context.webhooks.get(&raw.webhook).cloned().ok_or(ConfigError { message: format!("webhook '{}' does not exist", raw.webhook)})?,
            color: if let Some(hex) = raw.color { HexColor::parse(&hex)? } else { HexColor::rgb(0x80, 0x80, 0x80) },
            role_mentions,
            user_mentions,
            links,
            content_limit: raw.content_limit
        })
    }

    pub fn matches(&self, data: &EventData) -> bool {
        if !self.event.matches(data.name) { return false; }

        if let Some((name, is_wa)) = &data.nation {
            if !self.nation.matches(name) { return false; }
            if let Some(wa) = self.wa && *is_wa != wa { return false; }
        }

        if let Some(region) = &data.region && !self.region.matches(region) { return false; }
        if let Some(content) = &data.content && !self.content.matches(content) { return false; }

        return true;
    }
}

#[derive(Debug, Clone)]
enum Condition {
    String(String),
    Regex(Regex)
}

impl Condition {
    pub fn matches(&self, string: &str) -> bool {
        match self {
            Self::String(s) => s == string,
            Self::Regex(r) => r.is_match(string),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConditionList {
    include: Option<Vec<Condition>>,
    exclude: Option<Vec<Condition>>,
}

fn map_to_condition(v: &Vec<String>) -> Result<Vec<Condition>, Box<dyn Error>> {
    let mut result = Vec::new();
    for item in v {
        if let Some(pattern) = item.strip_prefix("#") { 
            result.push(Condition::Regex(Regex::new(pattern)?));
        } else {
            result.push(Condition::String(item.to_string()));
        }
    }

    Ok(result)
}

impl ConditionList {
    pub fn new(
        include: Option<Vec<String>>,
        exclude: Option<Vec<String>>
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            include: if let Some(v) = include { Some(map_to_condition(&v)?) } else { None },
            exclude: if let Some(v) = exclude { Some(map_to_condition(&v)?) } else { None },
        })
    }

    pub fn matches(&self, string: &str) -> bool {
        if let Some(v) = &self.exclude {
            for item in v {
                if item.matches(string) { return false; }
            }
        }

        if let Some(v) = &self.include {
            for item in v {
                if item.matches(string) { return true; }
            }

            return false;
        }

        return true;
    }
}