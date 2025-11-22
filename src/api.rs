use log::{error, info, warn};
use std::{collections::HashSet, time::Duration};

use caramel::ns::api::{Client, ApiError};
use caramel::ns::xml::{parse_wa_members, parse_rmb_posts};
use caramel::types::ns::Post;

pub async fn make_request_with_retry_loop(
    client: &Client, params: Vec<(&str, &str)>
) -> Result<String, ApiError> {
    loop {
        match client.make_request(params.clone()).await {
            Ok(response) => return Ok(response),
            Err(err) => {
                match err {
                    ApiError::RateLimit(duration) => tokio::time::sleep(duration).await,
                    ApiError::TimedOut => tokio::time::sleep(Duration::from_secs(20)).await,
                    _ => {
                        error!("Error making API request");
                        return Err(err);
                    }
                }
            }
        }
    }
}

pub async fn query_wa_nations(
    client: &Client, set: &mut HashSet<String>
) -> Result<(), ApiError> {
    let response = make_request_with_retry_loop(client, vec![
            ("wa", "1"), ("q", "members")
        ]).await?;

    if let Ok(members) = parse_wa_members(&response) {
        set.clear();
        for nation in members {
            set.insert(nation);
        }

        info!("Queried {} WA nations", set.len());
    } else {
        warn!("Invalid XML from WA members API request");
    }

    return Ok(());
}

pub async fn query_rmb_posts(
    client: &Client, region: &str, fromid: u64, limit: u64
) -> Result<Vec<Post>, ApiError> {
    let response = make_request_with_retry_loop(client, vec![
            ("region", region), ("q", "messages"), ("fromid", &fromid.to_string()), 
            ("limit", &limit.min(100).max(1).to_string())
        ]).await?;

    if let Ok(posts) = parse_rmb_posts(&response) {
        info!("Queried {} posts from {}'s RMB", posts.len(), region);
        return Ok(posts);
    } else {
        warn!("Invalid XML from RMB posts API request");
        return Ok(vec![]);
    }
}