use std::{collections::HashMap, sync::LazyLock};

use caramel::{ns::UserAgent, types::akari::Event};
use serenity::all::CreateButton;

use crate::{events::EventData, nscode, utils::chamber_link};

pub type LinkGenerator = fn (&Event, &EventData, &UserAgent) -> Option<CreateButton>;

fn create_link_map() -> HashMap<&'static str, LinkGenerator> {
    let mut link_map: HashMap<&'static str, LinkGenerator> = HashMap::new();

    link_map.insert("endorse", generate_endorse_link);
    link_map.insert("vote", generate_vote_link);
    link_map.insert("post", generate_post_link);
    link_map.insert("quote", generate_quote_link);

    link_map
}

pub static LINK_MAP: LazyLock<HashMap<&'static str, LinkGenerator>> = LazyLock::new(|| create_link_map());

fn generate_endorse_link(_: &Event, data: &EventData, user_agent: &UserAgent) -> Option<CreateButton> {
    if let Some((nation, _)) = &data.nation {
        Some(CreateButton::new_link(
            format!("https://www.nationstates.net/nation={}?generated_by={}#endorse", 
                nation, user_agent.web()
            )
        ).label("Endorse Nation"))
    } else {
        None
    }
}

fn generate_vote_link(event: &Event, data: &EventData, user_agent: &UserAgent) -> Option<CreateButton> {
    if data.name == "wa-floor" {
        Some(CreateButton::new_link(
            format!("{}?generated_by={}", 
                chamber_link(&event.data[0]), user_agent.web()
            )
        ).label("Open Voting Page"))
    } else {
        None
    }
}

fn generate_post_link(event: &Event, data: &EventData, user_agent: &UserAgent) -> Option<CreateButton> {
    if data.name == "rmb" {
        let region = event.origin.as_ref().unwrap();
        let postid = &event.data[0];

        Some(CreateButton::new_link(
            format!(
                "https://www.nationstates.net/page=display_region_rmb/region={}?generated_by={}&postid={}#p{}", 
                region, user_agent.web(), postid, postid
            )
        ).label("View Full Post"))
    } else {
        None
    }
}

const MAX_DISCORD_URL_LENGTH: usize = 512;

fn generate_quote_link_inner(
    region: &str,
    nation: &str,
    postid: &str,
    quote_content: &str,
    user_agent: &UserAgent
) -> String {
    let quote = format!("[quote={};{}]{}[/quote]\n", nation, postid, quote_content);

    let url = format!(
        "https://www.nationstates.net/page=display_region_rmb/region={}?generated_by={}&message={}#editor", 
        region, user_agent.web(), urlencoding::encode(&quote).into_owned()
    );

    if url.len() >= MAX_DISCORD_URL_LENGTH {
        return generate_quote_link_inner(region, nation, postid, "- snip -", user_agent);
    }

    url
}

fn generate_quote_link(event: &Event, data: &EventData, user_agent: &UserAgent) -> Option<CreateButton> {
    if data.name == "rmb" {
        let nation = event.actor.as_ref().unwrap();
        let region = event.origin.as_ref().unwrap();
        let postid = &event.data[0];
        let message = &event.data[1];
        let quote_content = nscode::remove_subquotes(message);

        Some(CreateButton::new_link(
            generate_quote_link_inner(region, nation, postid, &quote_content, user_agent)
        ).label("Quote Post"))
    } else {
        None
    }
}