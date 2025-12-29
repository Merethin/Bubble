use log::{info, warn};
use std::{collections::HashSet};

use caramel::ns::api::{Client, ApiError};
use caramel::ns::xml::{parse_rmb_posts, parse_wa_members, parse_world_regions};
use caramel::types::ns::Post;

pub async fn query_wa_nations(
    client: &Client, set: &mut HashSet<String>
) -> Result<(), ApiError> {
    let response = client.make_request_with_retry(vec![
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

pub async fn query_regions_by_tag(
    client: &Client, set: &mut HashSet<String>, params: Vec<String>
) -> Result<(), ApiError> {
    let response = client.make_request_with_retry(vec![
        ("q", "regionsbytag"), ("tags", &params.join(","))
    ]).await?;

    if let Ok(regions) = parse_world_regions(&response) {
        set.clear();
        for nation in regions {
            set.insert(nation);
        }

        info!("Queried {} regions with tags {:?}", set.len(), params);
    } else {
        warn!("Invalid XML from regionsbytag API request");
    }

    return Ok(());
}

pub async fn query_rmb_posts(
    client: &Client, region: &str, fromid: u64, limit: u64
) -> Result<Vec<Post>, ApiError> {
    let response = client.make_request_with_retry(vec![
            ("region", region), ("q", "messages"), ("fromid", &fromid.to_string()), 
            ("limit", &limit.clamp(1, 100).to_string())
        ]).await?;

    if let Ok(posts) = parse_rmb_posts(&response) {
        info!("Queried {} posts from {}'s RMB", posts.len(), region);
        return Ok(posts);
    }
    
    warn!("Invalid XML from RMB posts API request");
    return Ok(vec![]);
}