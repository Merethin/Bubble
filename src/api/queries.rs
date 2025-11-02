use crate::api::{ApiError, client::Client};
use serde::Deserialize;
use quick_xml::de::from_str;

use log::{error, info, warn};
use std::{collections::HashSet, time::Duration};

#[derive(Deserialize)]
struct WaMemberRoot {
    #[serde(rename = "MEMBERS")]
    members: String,
}

pub async fn query_wa_nations(client: &Client, set: &mut HashSet<String>) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match client.make_request(vec![
            ("wa", "1"), ("q", "members")
        ]).await {
            Ok(xml) => {
                if let Ok(root) = from_str::<WaMemberRoot>(&xml) {
                    set.clear();
                    for v in root.members.split(",") {
                        set.insert(v.to_string());
                    }
                    info!("Queried {} WA nations from the API", set.len());
                    return Ok(());
                } else {
                    warn!("Invalid XML from WA members API request");
                    return Ok(());
                }
            },
            Err(err) => {
                match err {
                    ApiError::RateLimit(duration) => tokio::time::sleep(duration).await,
                    ApiError::TimedOut => tokio::time::sleep(Duration::from_secs(10)).await,
                    _ => {
                        error!("Error requesting WA members");
                        return Err(Box::new(err));
                    }
                }
            }
        }
    }
}