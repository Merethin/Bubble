use serenity::all::Http;
use tokio::sync::{mpsc, RwLock};
use std::{collections::HashSet, sync::Arc, time::Duration};

use caramel::ns::api::Client;

use crate::{api, rmb, config::Config};

pub fn spawn_wa_worker(
    client: Arc<Client>,
    wa_nations: Arc<RwLock<HashSet<String>>>,
) -> mpsc::Sender<()> {
    let (send, mut recv) = mpsc::channel::<()>(100);

    let _ = tokio::spawn(async move {
        while let Some(_) = recv.recv().await {
            loop {
                let mut wa_nations = wa_nations.write().await;

                if let Err(_) = api::query_wa_nations(&client, &mut wa_nations).await {
                    drop(wa_nations);
                    tokio::time::sleep(Duration::from_secs(120)).await; // Try again after 2 minutes
                } else {
                    break;
                }
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

    let _ = tokio::spawn(async move {
        let http = Http::new("");

        loop {
            // Eagerly fetch posts until we've got everything pending, don't block
            while let Ok(post) = recv.try_recv() {
                rmb::enqueue_post(&mut queues, post);
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
                rmb::enqueue_post(&mut queues, post);
            }
        }
    });

    send
}