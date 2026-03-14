use serenity::all::{CreateButton, Http};
use std::error::Error;
use html_escape::decode_html_entities;

use caramel::{ns::{UserAgent, format::prettify_name}, types::akari::Event};

use crate::webhook::{build_event_embed, send_embed_to_webhook};
use crate::{config::OutputConfig, nscode};

fn encode_unicode_as_html_entities(input: &str) -> String {
    input.chars()
         .map(|c| if c as u32 > 127 { format!("&#{};", c as u32) } else { c.to_string() })
         .collect()
}

fn generate_quote_link(
    region: &str,
    nation: &str,
    postid: &str,
    quote_content: &str,
    user_agent: &UserAgent
) -> String {
    let quote = format!("[quote={};{}]{}[/quote]\n", nation, postid, quote_content);

    let url = format!(
        "https://www.nationstates.net/page=display_region_rmb/region={}?generated_by={}&message={}#editor", 
        region, user_agent.web(), 
        urlencoding::encode(&encode_unicode_as_html_entities(&quote)).into_owned()
    );

    if url.len() >= 512 {
        return generate_quote_link(region, nation, postid, "- snip -", user_agent);
    }

    url
}

pub async fn output_rmb_post(
    http: &Http,
    output_config: &OutputConfig,
    event: &Event,
    user_agent: &UserAgent
) -> Result<(), Box<dyn Error>> {
    let nation = event.actor.as_ref().unwrap();
    let region = event.origin.as_ref().unwrap();
    let postid = &event.data[0];
    let message = &event.data[1];

    let (content, quote_content) = format_content(message);

    let mut buttons: Vec<CreateButton> = Vec::new();
    
    buttons.push(
        CreateButton::new_link(
            format!(
                "https://www.nationstates.net/page=display_region_rmb/region={}?generated_by={}&postid={}#p{}", 
                region, user_agent.web(), postid, postid
            )
        ).label("View Post")
    );

    buttons.push(
        CreateButton::new_link(
        generate_quote_link(region, nation, postid, &quote_content, user_agent)
        ).label("Quote Post")
    );

    let footer = format!("Posted by {}", prettify_name(&nation));

    let embed = build_event_embed(
        output_config.color, &content, event.time,Some(&footer)
    )?.title(
        format!("New post on {}'s RMB", prettify_name(&region))
    );

    send_embed_to_webhook(
        http,
        &output_config.hook,
        output_config.mentions.clone(),
        embed,
        buttons
    ).await
}

pub fn format_content(
    content: &String
) -> (String, String) {
    let decoded = decode_html_entities(content).into_owned();

    let quote_content = nscode::remove_subquotes(&decoded);

    if let Some(tags) = nscode::parse(&decoded) {
        let fmt = nscode::render(tags, 4096);

        return (fmt, quote_content);
    }

    ("**Error: unable to parse RMB post, view the post by clicking the 'View Post' button**".into(), quote_content)
}