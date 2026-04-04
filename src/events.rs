use std::sync::Arc;

use crate::cache::NSCache;
use caramel::types::akari::Event;

pub struct EventData {
    pub name: &'static str,
    pub nation: Option<String>,
    pub region: Option<String>,
}

pub async fn classify_event(event: &Event, cache: Arc<NSCache>) -> Option<Vec<EventData>> {
    match event.category.as_str() {
        "ncte" => {
            let mut wa_nations = cache.wa_nations.write().await;
            let nation = event.receptor.as_ref()?;
            let is_wa = wa_nations.remove(nation);
            Some(vec![EventData { name: if is_wa { "wacte" } else { "cte" }, nation: Some(nation.clone()), region: Some(event.origin.as_ref()?.clone()) }])
        },
        "wadmit" => {
            let nation = event.actor.as_ref()?;
            cache.wa_nations.write().await.insert(nation.clone());
            Some(vec![EventData { name: "admit", nation: Some(nation.clone()), region: Some(event.origin.as_ref()?.clone()) }])
        },
        "wresign" => {
            let nation = event.actor.as_ref()?;
            cache.wa_nations.write().await.remove(nation);
            Some(vec![EventData { name: "resign", nation: Some(nation.clone()), region: Some(event.origin.as_ref()?.clone()) }])
        },
        "wkick" => {
            let nation = event.actor.as_ref()?;
            cache.wa_nations.write().await.remove(nation);
            Some(vec![EventData { name: "kick", nation: Some(nation.clone()), region: Some(event.origin.as_ref()?.clone()) }])
        },
        "move" => {
            let nation = event.actor.as_ref()?;
            let is_wa = cache.wa_nations.read().await.contains(nation);
            Some(vec![
                EventData { name: if is_wa { "wajoin" } else { "join" }, nation: Some(nation.clone()), region: Some(event.destination.as_ref()?.clone()) },
                EventData { name: if is_wa { "waleave" } else { "leave" }, nation: Some(nation.clone()), region: Some(event.origin.as_ref()?.clone()) }
            ])
        },
        "rmbpost" => Some(vec![EventData { 
            name: "rmb", 
            nation: Some(event.actor.as_ref()?.clone()), 
            region: Some(event.origin.as_ref()?.clone()) 
        }]),
        "rupdate" => Some(vec![EventData { 
            name: "update", nation: None,
            region: Some(event.origin.as_ref()?.clone()) 
        }]),
        "rfeature" | "rmapfeat" => Some(vec![EventData { 
            name: "feature", nation: None,
            region: Some(event.origin.as_ref()?.clone()) 
        }]),
        "ndel" | "rdel" | "ldel" => Some(vec![EventData { 
            name: "delegate",
            nation: Some(event.receptor.as_ref()?.clone()), 
            region: Some(event.origin.as_ref()?.clone()) 
        }]),
        "nfound" | "nrefound" => Some(vec![EventData { 
            name: "found",
            nation: Some(event.actor.as_ref()?.clone()), 
            region: Some(event.origin.as_ref()?.clone()) 
        }]),
        "wapply" => Some(vec![EventData { 
            name: "apply",
            nation: Some(event.actor.as_ref()?.clone()), 
            region: Some(event.origin.as_ref()?.clone()) 
        }]),
        "rsfloor" => Some(vec![EventData { name: "wa-floor", nation: None, region: None }]),
        "rssubmit" => Some(vec![EventData { name: "wa-submit", nation: None, region: None }]),
        "rspass" => Some(vec![EventData { name: "wa-pass", nation: None, region: None }]),
        "rsfail" => Some(vec![EventData { name: "wa-fail", nation: None, region: None }]),
        "rdiscard" => Some(vec![EventData { name: "wa-discard", nation: None, region: None }]),
        _ => {
            Some(vec![])
        }
    }
}

pub async fn check_and_update_tag_cloud(event: &Event, cache: Arc<NSCache>) {
    match event.category.as_str() {
        "rgcte" | "govabd" => {
            if let Some(region) = &event.origin {
                if let Some(tag_entry) = cache.tag_cloud.write().await.get_mut("governorless") {
                    tag_entry.insert(region.clone());
                }
            }
        },
        "fngovrem" => {
            if let Some(region) = &event.origin {
                let mut tag_cloud = cache.tag_cloud.write().await;
                if let Some(tag_entry) = tag_cloud.get_mut("governorless") {
                    tag_entry.insert(region.clone());
                }
                if let Some(tag_entry) = tag_cloud.get_mut("frontier") {
                    tag_entry.insert(region.clone());
                }
            }
        }
        "rnewgov" => {
            if let Some(region) = &event.origin {
                if let Some(tag_entry) = cache.tag_cloud.write().await.get_mut("governorless") {
                    tag_entry.remove(region);
                }
            }
        }
        "stgovadd" => {
            if let Some(region) = &event.origin {
                let mut tag_cloud = cache.tag_cloud.write().await;
                if let Some(tag_entry) = tag_cloud.get_mut("governorless") {
                    tag_entry.remove(region);
                }
                if let Some(tag_entry) = tag_cloud.get_mut("frontier") {
                    tag_entry.remove(region);
                }
            }
        },
        "addtag" => {
            if let Some(region) = &event.origin
            && let Some(tag) = event.data.get(0).map(
                |v| v.to_lowercase().replace(' ', "_")
            ) {
                if let Some(tag_entry) = cache.tag_cloud.write().await.get_mut(&tag) {
                    tag_entry.insert(region.clone());
                }
            }
        },
        "rmtag" => {
            if let Some(region) = &event.origin
            && let Some(tag) = event.data.get(0).map(
                |v| v.to_lowercase().replace(' ', "_")
            ) {
                if let Some(tag_entry) = cache.tag_cloud.write().await.get_mut(&tag) {
                    tag_entry.remove(region);
                }
            }
        },
        "rfound" => {
            cache.tick_tag_query().await;
        }
        _ => {}
    }
}