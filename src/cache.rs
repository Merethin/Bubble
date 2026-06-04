use std::{collections::HashSet, sync::Arc};
use tokio::sync::RwLock;

pub struct NSCache {
    pub wa_nations: RwLock<HashSet<String>>,
}

impl NSCache {
    pub fn new() -> Arc<Self> {
        Arc::new(
            Self {
                wa_nations: RwLock::new(HashSet::new()),
            }
        )
    }
}