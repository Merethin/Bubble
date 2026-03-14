use tokio::sync::mpsc;
use std::{collections::HashSet, sync::Arc, time::Duration};

use caramel::ns::api::Client;

use crate::{api, cache::NSCache};

pub enum NSQuery {
    UpdateWA,
    UpdateTag(String),
}

pub fn spawn_ns_worker(
    client: Arc<Client>,
    cache: Arc<NSCache>,
) -> mpsc::Sender<NSQuery> {
    let (send, mut recv) = mpsc::channel::<NSQuery>(100);

    tokio::spawn(async move {
        while let Some(query) = recv.recv().await {
            match query {
                NSQuery::UpdateWA => loop {
                    let mut wa_nations = cache.wa_nations.write().await;

                    if let Err(_) = api::query_wa_nations(&client, &mut wa_nations).await {
                        drop(wa_nations);
                        tokio::time::sleep(Duration::from_secs(120)).await; // Try again after 2 minutes
                    } else {
                        break;
                    }
                },
                NSQuery::UpdateTag(tag) => loop {
                    let mut tag_cloud = cache.tag_cloud.write().await;

                    let mut tag_entry = tag_cloud.entry(tag.clone()).or_insert(
                        HashSet::new()
                    );

                    if let Err(_) = api::query_regions_by_tag(&client, &mut tag_entry, vec![tag.clone()]).await {
                        drop(tag_cloud);
                        tokio::time::sleep(Duration::from_secs(120)).await; // Try again after 2 minutes
                    } else {
                        break;
                    }
                },
            }
        }
    });

    send
}