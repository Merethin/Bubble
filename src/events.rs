use std::sync::Arc;

use crate::cache::NSCache;
use caramel::types::akari::Event;

pub struct EventData {
    pub name: &'static str,
    pub nation: Option<(String, bool)>,
    pub region: Option<String>,
    pub content: Option<String>,
}

pub async fn classify_event(event: &Event, cache: Arc<NSCache>) -> Option<Vec<EventData>> {
    match event.category.as_str() {
        "ncte" => {
            let mut wa_nations = cache.wa_nations.write().await;
            let nation = event.receptor.as_ref()?;
            let is_wa = wa_nations.remove(nation);
            Some(vec![EventData { name: "cte", nation: Some((nation.clone(), is_wa)), region: Some(event.origin.as_ref()?.clone()), content: None }])
        },
        "wadmit" => {
            let nation = event.actor.as_ref()?;
            cache.wa_nations.write().await.insert(nation.clone());
            Some(vec![EventData { name: "admit", nation: Some((nation.clone(), true)), region: Some(event.origin.as_ref()?.clone()), content: None }])
        },
        "wresign" => {
            let nation = event.actor.as_ref()?;
            cache.wa_nations.write().await.remove(nation);
            Some(vec![EventData { name: "resign", nation: Some((nation.clone(), false)), region: Some(event.origin.as_ref()?.clone()), content: None }])
        },
        "wkick" => {
            let nation = event.actor.as_ref()?;
            cache.wa_nations.write().await.remove(nation);
            Some(vec![EventData { name: "kick", nation: Some((nation.clone(), false)), region: Some(event.origin.as_ref()?.clone()), content: None }])
        },
        "move" => {
            let nation = event.actor.as_ref()?;
            let is_wa = cache.wa_nations.read().await.contains(nation);
            Some(vec![
                EventData { name: "join", nation: Some((nation.clone(), is_wa)), region: Some(event.destination.as_ref()?.clone()), content: None },
                EventData { name: "leave", nation: Some((nation.clone(), is_wa)), region: Some(event.origin.as_ref()?.clone()), content: None }
            ])
        },
        "rmbpost" => {
            let nation = event.actor.as_ref()?;
            let is_wa = cache.wa_nations.read().await.contains(nation);
            Some(vec![EventData { 
                name: "rmb", 
                nation: Some((nation.clone(), is_wa)), 
                region: Some(event.origin.as_ref()?.clone()), 
                content: Some(event.data[1].clone())
            }])
        },
        "rupdate" => Some(vec![EventData { 
            name: "update", nation: None,
            region: Some(event.origin.as_ref()?.clone()), content: None
        }]),
        "rfeature" | "rmapfeat" => Some(vec![EventData { 
            name: "feature", nation: None,
            region: Some(event.origin.as_ref()?.clone()), content: None
        }]),
        "ndel" | "rdel" => Some(vec![EventData { 
            name: "delegate",
            nation: Some((event.receptor.as_ref()?.clone(), true)), 
            region: Some(event.origin.as_ref()?.clone()), content: None
        }]),
        "ldel" => {
            let nation = event.receptor.as_ref()?;
            let is_wa = cache.wa_nations.read().await.contains(nation);
            Some(vec![EventData { 
                name: "delegate",
                nation: Some((nation.clone(), is_wa)), 
                region: Some(event.origin.as_ref()?.clone()), content: None
            }])
        },
        "nfound" | "nrefound" => Some(vec![EventData { 
            name: "found",
            nation: Some((event.actor.as_ref()?.clone(), false)), 
            region: Some(event.origin.as_ref()?.clone()), content: None
        }]),
        "wapply" => Some(vec![EventData { 
            name: "apply",
            nation: Some((event.actor.as_ref()?.clone(), false)), 
            region: Some(event.origin.as_ref()?.clone()), content: None
        }]),
        "rsfloor" => Some(vec![EventData { name: "wa-floor", nation: None, region: None, content: None }]),
        "rssubmit" => Some(vec![EventData { name: "wa-submit", nation: None, region: None, content: None }]),
        "rspass" => Some(vec![EventData { name: "wa-pass", nation: None, region: None, content: None }]),
        "rsfail" => Some(vec![EventData { name: "wa-fail", nation: None, region: None, content: None }]),
        "rdiscard" => Some(vec![EventData { name: "wa-discard", nation: None, region: None, content: None }]),
        _ => {
            Some(vec![])
        }
    }
}