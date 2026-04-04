use std::{collections::HashMap, sync::LazyLock};

use itertools::Itertools;
use log::warn;
use serenity::all::{CreateButton, Http};

use caramel::ns::UserAgent;
use caramel::types::akari::Event;

use crate::config::OutputConfig;
use crate::rmb::output_rmb_post;
use crate::webhook::{build_event_embed, send_embed_to_webhook};
use crate::utils::{chamber_link, display_chamber, display_nation, display_proposal_name, display_proposal_url, display_region};

pub enum Field {
    Actor,
    Receptor,
    Origin,
    HighlightOrigin,
    Destination,
    HighlightDestination,
    Text(&'static str)
}

pub type ProcessorExtFn = fn(&Event) -> Option<String>;

use Field::{Actor, Receptor, Origin, Destination, HighlightOrigin, HighlightDestination, Text};

pub struct Processor {
    fields: Vec<Field>,
    custom: Option<ProcessorExtFn>
}

impl Processor {
    pub fn process(&self, event: &Event) -> Option<String> {
        let mut result: Vec<String> = Vec::new();

        for field in &self.fields {
            match field {
                Actor => result.push(display_region(event.actor.as_ref()?, false)),
                Receptor => result.push(display_region(event.receptor.as_ref()?, false)),
                Origin => result.push(display_region(event.origin.as_ref()?, false)),
                Destination => result.push(display_region(event.destination.as_ref()?, false)),
                HighlightOrigin => result.push(display_region(event.origin.as_ref()?, true)),
                HighlightDestination => result.push(display_region(event.destination.as_ref()?, true)),
                Text(text) => result.push((*text).to_string()),
            }
        }

        if let Some(func) = self.custom {
            result.push(func(&event)?);
        }

        Some(result.join(""))
    }

    pub fn init(fields: Vec<Field>, custom: ProcessorExtFn) -> Self {
        Processor { fields, custom: Some(custom) }
    }
}

impl From<Vec<Field>> for Processor {
    fn from(fields: Vec<Field>) -> Self {
        Self { fields, custom: None }
    }
}

pub type OutputMap = HashMap<&'static str, Processor>;

fn process_found(event: &Event) -> Option<String> {
    let actor = event.actor.as_ref()?;
    let origin = event.origin.as_ref()?;

    if event.category == "nfound" {
        Some(format!("{} was founded in {}", 
            display_nation(actor, true),
            display_region(origin, true)
        ))
    } else {
        Some(format!("{} was refounded in {}", 
            display_nation(actor, true),
            display_region(origin, true)
        ))
    }
}

fn process_delegate(event: &Event) -> Option<String> {
    let receptor = event.receptor.as_ref()?;
    let origin = event.origin.as_ref()?;

    if event.category == "ndel" {
        Some(format!("{} became WA delegate of {}", 
            display_nation(receptor, true),
            display_region(origin, true)
        ))
    } else if event.category == "rdel" {
        let old_delegate = event.data.get(0)?;
        Some(format!("{} seized the delegacy of {} from {}", 
            display_nation(receptor, true),
            display_region(origin, true),
            display_nation(old_delegate, false)
        ))
    } else {
        Some(format!("{} lost WA delegate status in {}", 
            display_nation(receptor, true),
            display_region(origin, true)
        ))
    }
}

fn process_wa_floor(event: &Event) -> Option<String> {
    let author = event.receptor.as_ref()?;
    let chamber = event.data.get(0)?;
    let proposal = event.data.get(1)?;

    if let Some((_, coauthors)) = event.data.split_at_checked(2) && !coauthors.is_empty() {
        Some(format!("The {} resolution {} (by {}, coauthor(s): {}) is now at vote",
            display_chamber(chamber, true),
            display_proposal_name(proposal), 
            display_nation(author, true),
            coauthors.iter().map(|nation| display_nation(nation, false)).join(", ")
        ))
    } else {
        Some(format!("The {} resolution {} (by {}) is now at vote", 
            display_chamber(chamber, true),
            display_proposal_name(proposal), 
            display_nation(author, true),
        ))
    }
}

fn process_wa_submit(event: &Event) -> Option<String> {
    let author = event.actor.as_ref().unwrap();
    let chamber = event.data.get(0)?;
    let board = event.data.get(1)?;
    let proposal = event.data.get(2)?;

    Some(format!("{} submitted a proposal ({}) to the {} {} Board", 
        display_nation(author, true),
        display_proposal_name(proposal), 
        chamber, board
    ))
}

fn process_wa_pass(event: &Event) -> Option<String> {
    let chamber = event.data.get(0)?;
    let resolution = event.data.get(1)?;
    let proposal = event.data.get(2)?;
    let votes_for = event.data.get(3)?;
    let votes_against = event.data.get(4)?;

    Some(format!("The {} resolution {} was passed {} votes FOR to {} votes AGAINST", 
        display_chamber(chamber, false),
        display_proposal_url(proposal, chamber, resolution, true),
        votes_for,
        votes_against
    ))
}

fn process_wa_fail(event: &Event) -> Option<String> {
    let chamber = event.data.get(0)?;
    let proposal = event.data.get(1)?;
    let votes_against = event.data.get(2)?;
    let votes_for = event.data.get(3)?;

    Some(format!("The {} resolution {} was defeated {} votes AGAINST to {} votes FOR", 
        display_chamber(chamber, false),
        display_proposal_name(proposal),
        votes_against,
        votes_for
    ))
}

fn process_wa_discard(event: &Event) -> Option<String> {
    let chamber = event.data.get(0)?;
    let proposal = event.data.get(1)?;
    let votes_for = event.data.get(2)?;
    let votes_against = event.data.get(3)?;

    Some(format!("The {} resolution {} was discarded after getting {} votes FOR and {} votes AGAINST", 
        display_chamber(chamber, false),
        display_proposal_name(proposal),
        votes_for,
        votes_against
    ))
}

fn create_output_map() -> OutputMap {
    let mut line_map = HashMap::new();

    line_map.insert("join", vec![
        Actor, Text(" relocated from "), Origin, Text(" to "), HighlightDestination
    ].into());
    line_map.insert("wajoin", vec![
        Actor, Text(" **(WA)** relocated from "), Origin, Text(" to "), HighlightDestination
    ].into());
    line_map.insert("leave", vec![
        Actor, Text(" relocated from "), HighlightOrigin, Text(" to "), Destination
    ].into());
    line_map.insert("waleave", vec![
        Actor, Text(" **(WA)** relocated from "), HighlightOrigin, Text(" to "), Destination
    ].into());
    line_map.insert("cte", vec![
        Receptor, Text(" ceased to exist in "), HighlightOrigin
    ].into());
    line_map.insert("wacte", vec![
        Receptor, Text(" **(WA)** ceased to exist in "), HighlightOrigin
    ].into());
    line_map.insert("admit", vec![
        Actor, Text(" was admitted to the World Assembly in "), HighlightOrigin
    ].into());
    line_map.insert("resign", vec![
        Actor, Text(" resigned from the World Assembly in "), HighlightOrigin
    ].into());
    line_map.insert("apply", vec![
        Actor, Text(" applied to join the World Assembly in "), HighlightOrigin
    ].into());
    line_map.insert("wakick", vec![
        Receptor, Text(" was ejected from the World Assembly for rule violations in "), HighlightOrigin
    ].into());
    line_map.insert("update", vec![
        HighlightOrigin, Text(" updated")
    ].into());
    line_map.insert("feature", vec![
        HighlightOrigin, Text(" became the Featured Region of the day")
    ].into());
    line_map.insert("found", Processor::init(vec![], process_found));
    line_map.insert("delegate", Processor::init(vec![], process_delegate));
    line_map.insert("wa-floor", Processor::init(vec![], process_wa_floor));
    line_map.insert("wa-submit", Processor::init(vec![], process_wa_submit));
    line_map.insert("wa-pass", Processor::init(vec![], process_wa_pass));
    line_map.insert("wa-fail", Processor::init(vec![], process_wa_fail));
    line_map.insert("wa-discard", Processor::init(vec![], process_wa_discard));

    line_map
}

static OUTPUT_MAP: LazyLock<OutputMap> = LazyLock::new(|| create_output_map());

pub async fn output_event(
    http: &Http,
    category: &str,
    output_config: &OutputConfig,
    event: &Event,
    user_agent: &UserAgent
) -> Result<(), Box<dyn std::error::Error>> {  
    if category == "rmb" {
        output_rmb_post(http, output_config, event, user_agent).await?;

        return Ok(());
    } 

    if let Some(processor) = OUTPUT_MAP.get(category) {
        let Some(description) = processor.process(event) else {
            warn!("Event {} is missing fields: {:?}", event.category, event);
            return Ok(());
        };

        let mut buttons: Vec<CreateButton> = Vec::new();
        
        if category == "wajoin" || category == "admit" {
            buttons.push(
                CreateButton::new_link(
                    format!("https://www.nationstates.net/nation={}?generated_by={}#endorse", 
                        event.actor.as_ref().unwrap(), user_agent.web()
                    )
                ).label("Endorse Nation")
            );
        }

        if category == "wa-floor" {
            buttons.push(
                CreateButton::new_link(
                    format!("{}?generated_by={}", 
                        chamber_link(&event.data[0]), user_agent.web()
                    )
                ).label("Open Voting Page")
            );
        }

        let embed = build_event_embed(
            output_config.color, &description, event.time, None
        )?;

        send_embed_to_webhook(
            http, 
            &output_config.hook,
            output_config.mentions.clone(),
            embed,
            buttons
        ).await?;
    }

    Ok(())
}