use serenity::all::Http;
use tokio::sync::mpsc;
use std::{collections::HashSet, sync::Arc, time::Duration};

use caramel::ns::api::Client;

use crate::{api, cache::NSCache, config::Config, rmb};

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

pub fn spawn_rmb_worker(
    config: &Config, client: Arc<Client>
) -> mpsc::Sender<rmb::Post> {
    let (send, mut recv) = mpsc::channel::<rmb::Post>(100);

    let mut queues = rmb::create_rmb_queues(config);

    tokio::spawn(async move {
        let http = Http::new("");

        loop {
            // Eagerly fetch posts until we've got everything pending, don't block
            while let Ok(post) = recv.try_recv() {
                rmb::enqueue_post(&mut queues, &post);
            }

            // Go through each region, if any has pending posts, fetch them (only one at a time)
            let mut was_fetched: bool = false;
            for (region, queue) in rmb::sort_queues(&mut queues) {
                if rmb::fetch_posts_if_pending(&http, &client, region, queue).await {
                    was_fetched = true;
                    break;
                }
            }

            // If there were no posts, block
            if !was_fetched && let Some(post) = recv.recv().await {
                rmb::enqueue_post(&mut queues, &post);
            }
        }
    });

    send
}