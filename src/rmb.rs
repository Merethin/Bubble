use log::error;
use serenity::all::{CreateButton, Http};
use std::{collections::{HashMap, VecDeque}, time::Duration, error::Error};
use itertools::Itertools;
use html_escape::decode_html_entities;

use caramel::ns::{api::Client, UserAgent, format::prettify_name};
use caramel::types::ns::Post as RMBPost;

use crate::webhook::{build_event_embed, send_embed_to_webhook};
use crate::{api::query_rmb_posts, config::{Config, OutputConfig}, nscode};

pub type Post = (String, u64);

pub struct RegionQueue {
    pending_posts: VecDeque<u64>,
    config: OutputConfig,
}

impl RegionQueue {
    fn new(config: OutputConfig) -> Self {
        Self { pending_posts: VecDeque::new(), config: config }
    }

    fn has_posts_pending(&self) -> bool {
        !self.pending_posts.is_empty()
    }

    fn add_post(&mut self, postid: u64) {
        self.pending_posts.push_back(postid);
    }

    fn consume_post(&mut self, postid: u64) {
        if let Some(index) = self.pending_posts.iter().position(|p| *p == postid) {
            self.pending_posts.remove(index);
        }
    }

    fn get_starting_postid(&self) -> Option<u64> {
        self.pending_posts.get(0).cloned()
    }

    fn get_pending_posts(&self) -> usize {
        self.pending_posts.len()
    }
}

pub fn create_rmb_queues(config: &Config) -> HashMap<String, RegionQueue> {
    let mut map: HashMap<String, RegionQueue> = HashMap::new();
    for region in &config.regions {
        if let Some(e) = config.get_event(region.0, "rmb") {
            map.insert(region.0.clone(), RegionQueue::new(e.clone()));
        }
    }

    map
}

fn encode_unicode_as_html_entities(input: &str) -> String {
    input.chars()
         .map(|c| if c as u32 > 127 { format!("&#{};", c as u32) } else { c.to_string() })
         .collect()
}

fn generate_quote_link(
    region: &str,
    post: &RMBPost,
    quote_content: &str,
    user_agent: &UserAgent
) -> String {
    let quote = format!("[quote={};{}]{}[/quote]\n", post.nation, post.id, quote_content);

    let url = format!(
        "https://www.nationstates.net/page=display_region_rmb/region={}?generated_by={}&message={}#editor", 
        region, user_agent.web(), 
        urlencoding::encode(&encode_unicode_as_html_entities(&quote)).into_owned()
    );

    if url.len() >= 512 {
        return generate_quote_link(region, post, "- snip -", user_agent);
    }

    url
}

async fn output_rmb_post(
    region: &str,
    content: &str,
    post: &RMBPost,
    quote_content: &str,
    output_config: &OutputConfig,
    user_agent: &UserAgent,
    http: &Http
) -> Result<(), Box<dyn Error>> {
    let mut buttons: Vec<CreateButton> = Vec::new();
    
    buttons.push(
        CreateButton::new_link(
            format!(
                "https://www.nationstates.net/page=display_region_rmb/region={}?generated_by={}&postid={}#p{}", 
                region, user_agent.web(), post.id, post.id
            )
        ).label("View Post")
    );

    buttons.push(
        CreateButton::new_link(
        generate_quote_link(region, post, quote_content, user_agent)
        ).label("Quote Post")
    );

    let footer = 
        if let Some(embassy) = &post.embassy {
            format!("Posted by {} [{}]", prettify_name(&post.nation), prettify_name(embassy))
        } else {
            format!("Posted by {}", prettify_name(&post.nation))
        };

    let embed = build_event_embed(
        &output_config.color, &content, post.timestamp, 
        Some(&footer)
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

pub fn sort_queues(
    queues: &mut HashMap<String, RegionQueue>
) -> std::vec::IntoIter<(&std::string::String, &mut RegionQueue)> {
    queues.iter_mut().sorted_by(|a, b| {
        b.1.get_pending_posts().cmp(&a.1.get_pending_posts())
    })
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

const RMB_POST_DELAY: u64 = 30;

pub async fn fetch_posts_if_pending(
    http: &Http,
    client: &Client, 
    region: &String, 
    queue: &mut RegionQueue
) -> bool {
    if !queue.has_posts_pending() {
        return false;
    }

    if let Ok(result) = query_rmb_posts(
        &client, region, queue.get_starting_postid().unwrap(), 
        queue.get_pending_posts().try_into().unwrap()
    ).await {
        for post in result {
            queue.consume_post(post.id.parse().unwrap());

            if let Some(message) = &post.message {
                let (content, quote_content) = format_content(message);

                output_rmb_post(
                    region, &content, &post, &quote_content,
                    &queue.config, &client.user_agent, &http
                ).await.unwrap_or_else(|err| {
                    error!("Failed to send RMB post {post:?} to webhook: {err}");
                });
            }
        }

        tokio::time::sleep(Duration::from_secs(RMB_POST_DELAY)).await;

        return true;
    }

    false
}

pub fn enqueue_post(
    queues: &mut HashMap<String, RegionQueue>,
    post: Post
) {
    if let Some(queue) = queues.get_mut(&post.0) {
        queue.add_post(post.1);
    }
}