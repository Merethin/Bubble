use log::{info, warn};
use std::{collections::HashSet};

use caramel::ns::api::{Client, ApiError};
use caramel::ns::xml::{parse_wa_members, parse_world_regions};

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