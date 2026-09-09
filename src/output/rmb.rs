use serenity::all::{CreateButton, Http};
use std::error::Error;

use caramel::{ns::{UserAgent, format::prettify_name}, types::akari::Event};

use crate::{events::EventData, render::render_tags};
use crate::{config::ParsedRule, nscode};
use super::webhook::{build_event_embed, send_embed_to_webhook};

pub async fn output_rmb_post(
    http: &Http,
    rule: &ParsedRule,
    event: &Event,
    data: &EventData,
    user_agent: &UserAgent
) -> Result<(), Box<dyn Error>> {
    let nation = event.actor.as_ref().unwrap();
    let region = event.origin.as_ref().unwrap();
    let message = &event.data[1];
    let content = format_content(message, rule.content_limit);

    let mut buttons: Vec<CreateButton> = Vec::new();
    for generator in &rule.links {
        if let Some(button) = generator(event, data, user_agent) {
            buttons.push(button);
        }
    }

    let footer = format!("Posted by {}", prettify_name(&nation));

    let embed = build_event_embed(
        rule.color, &content, event.time, Some(&footer)
    )?.title(
        format!("New post on {}'s RMB", prettify_name(&region))
    );

    send_embed_to_webhook(
        http,
        &rule.webhook,
        rule.role_mentions.clone(),
        rule.user_mentions.clone(),
        embed,
        buttons
    ).await
}

const MAX_DISCORD_EMBED_CONTENT: usize = 4096;

pub fn format_content(
    content: &String,
    limit: Option<usize>,
) -> String {
    let clamped_limit = limit.unwrap_or(MAX_DISCORD_EMBED_CONTENT).min(MAX_DISCORD_EMBED_CONTENT);

    if let Some(tags) = nscode::parse(content) {
        let output = render_tags(tags, clamped_limit);
        if output.len() == clamped_limit && (output.len() + 3) <= MAX_DISCORD_EMBED_CONTENT {
            return output + "...";
        } else {
            return output;
        }
    }

    "**Error: unable to parse RMB post, view the post by clicking the 'View Full Post' button**".into()
}