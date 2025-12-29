use std::{collections::{HashMap, HashSet}, sync::Arc, time::Instant};
use tokio::sync::{RwLock, mpsc::Sender};
use log::error;

use crate::{config::Config, worker::NSQuery};

pub struct NSCache {
    pub wa_nations: RwLock<HashSet<String>>,
    pub tag_cloud: RwLock<HashMap<String, HashSet<String>>>,
    pub next_tag_query: RwLock<(Instant, usize)>,
}

const TAG_UPDATE_MIN_REGIONS: usize = 10;
const TAG_UPDATE_COOLDOWN: u64 = 60 * 30; // 30 minutes

impl NSCache {
    pub fn new() -> Arc<Self> {
        Arc::new(
            Self {
                wa_nations: RwLock::new(HashSet::new()),
                tag_cloud: RwLock::new(HashMap::new()),
                next_tag_query: RwLock::new((Instant::now(), 0))
            }
        )
    }

    pub async fn tick_tag_query(&self) {
        self.next_tag_query.write().await.1 += 1;
    }

    pub async fn should_run_tag_query(&self) -> bool {
        let query = self.next_tag_query.read().await;

        query.0.elapsed().as_secs() > TAG_UPDATE_COOLDOWN && query.1 > TAG_UPDATE_MIN_REGIONS
    }

    pub async fn run_tag_query(&self, sender: &mut Sender<NSQuery>, config: &Config) {
        let mut query = self.next_tag_query.write().await;
        query.0 = Instant::now();
        query.1 = 0;
        drop(query);

        for tag in config.tags.keys() {
            sender.send(NSQuery::UpdateTag(tag.clone())).await.unwrap_or_else(|err| {
                error!("Failed to trigger tag update: {err}");
            });
        }
    }
}