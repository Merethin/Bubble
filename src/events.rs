use std::sync::Arc;

use crate::cache::NSCache;
use caramel::types::akari::Event;

pub async fn check_and_update_wa(event: &Event, cache: Arc<NSCache>) -> bool {
    match event.category.as_str() {
        "ncte" => {
            let mut wa_nations = cache.wa_nations.write().await;
            event.receptor.as_ref().is_some_and(|nation| {
                wa_nations.remove(nation)
            })
        },
        "wadmit" => {
            if let Some(nation) = &event.actor {
                cache.wa_nations.write().await.insert(nation.clone());
            }
            true
        },
        "wresign" => {
            if let Some(nation) = &event.actor {
                cache.wa_nations.write().await.remove(nation);
            }
            false
        },
        "wkick" => {
            if let Some(nation) = &event.receptor {
                cache.wa_nations.write().await.remove(nation);
            }
            false
        },
        _ => {
            let wa_nations = cache.wa_nations.read().await;
            event.actor.as_ref().is_some_and(|nation| {
                wa_nations.contains(nation)
            })
        },
    }
}

pub fn match_origin_category(event: &Event, is_wa: bool) -> Option<&'static str> {
    Some(match event.category.as_str() {
        "rmbpost" => "rmb",
        "rupdate" => "update",
        "rfeature" => "feature",
        "ndel" | "rdel" | "ldel" => "delegate",
        "nfound" | "nrefound" => "found",
        "wapply" => "apply",
        "wadmit" => "admit",
        "wresign" => "resign",
        "wkick" => "kick",
        "move" => if is_wa { "waleave" } else { "leave" },
        "ncte" => if is_wa { "wacte" } else { "cte" },
        _ => {
            return None;
        },
    })
}

pub fn match_world_category(event: &Event, is_wa: bool) -> Option<&'static str> {
    Some(match event.category.as_str() {
        "rsfloor" => "wa-floor",
        "rssubmit" => "wa-submit",
        "rspass" => "wa-pass",
        "rsfail" => "wa-fail",
        "rdiscard" => "wa-discard",
        "rfeature" => "feature",
        "ndel" | "rdel" | "ldel" => "delegate",
        "nfound" | "nrefound" => "found",
        "wapply" => "apply",
        "wadmit" => "admit",
        "wresign" => "resign",
        "wkick" => "kick",
        "ncte" => if is_wa { "wacte" } else { "cte" },
        _ => {
            return None;
        },
    })
}