use std::collections::HashMap;

use serenity::all::{CreateButton, Http};

use caramel::ns::UserAgent;
use caramel::types::akari::Event;

use crate::config::OutputConfig;
use crate::webhook::{build_event_embed, send_embed_to_webhook};
use crate::utils::{display_nation, display_region};

pub enum Field {
    Actor,
    Receptor,
    Origin,
    HighlightOrigin,
    Destination,
    HighlightDestination,
    Text(&'static str)
}

pub type ProcessorExtFn = fn(&Event) -> String;

use Field::*;

pub struct Processor {
    fields: Vec<Field>,
    custom: Option<ProcessorExtFn>
}

impl Processor {
    pub fn process(&self, event: Event) -> String {
        let mut result: Vec<String> = Vec::new();

        for field in &self.fields {
            match field {
                Actor => {
                    if let Some(actor) = &event.actor {
                        result.push(display_nation(&actor, true));
                    }
                },
                Receptor => {
                    if let Some(receptor) = &event.receptor {
                        result.push(display_nation(&receptor, true));
                    }
                },
                Origin => {
                    if let Some(origin) = &event.origin {
                        result.push(display_region(&origin, false));
                    }
                },
                Destination => {
                    if let Some(destination) = &event.destination {
                        result.push(display_region(&destination, false));
                    }
                },
                HighlightOrigin => {
                    if let Some(origin) = &event.origin {
                        result.push(display_region(&origin, true));
                    }
                },
                HighlightDestination => {
                    if let Some(destination) = &event.destination {
                        result.push(display_region(&destination, true));
                    }
                },
                Text(text) => {
                    result.push(text.to_string())
                }
            }
        }

        if let Some(func) = self.custom {
            result.push(func(&event));
        }

        result.join("")
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

fn process_found(event: &Event) -> String {
    let actor = event.actor.as_ref().unwrap();
    let origin = event.origin.as_ref().unwrap();

    if event.category == "nfound" {
        format!("{} was founded in {}", 
            display_nation(actor, true),
            display_region(origin, true)
        )
    } else {
        format!("{} was refounded in {}", 
            display_nation(actor, true),
            display_region(origin, true)
        )
    }
}

fn process_delegate(event: &Event) -> String {
    let receptor = event.receptor.as_ref().unwrap();
    let origin = event.origin.as_ref().unwrap();

    if event.category == "ndel" {
        format!("{} became WA delegate of {}", 
            display_nation(receptor, true),
            display_region(origin, true)
        )
    } else if event.category == "rdel" {
        let old_delegate = event.data.get(0).unwrap();
        format!("{} seized the delegacy of {} from {}", 
            display_nation(receptor, true),
            display_region(origin, true),
            display_nation(old_delegate, false)
        )
    } else {
        format!("{} lost WA delegate status in {}", 
            display_nation(receptor, true),
            display_region(origin, true)
        )
    }
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

    line_map
}

lazy_static::lazy_static! {
    static ref OUTPUT_MAP: OutputMap = create_output_map();
}

pub async fn output_event(
    http: &Http,
    category: &str,
    output_config: &OutputConfig,
    event: &Event,
    user_agent: &UserAgent
) -> Result<(), Box<dyn std::error::Error>> {   
    if let Some(processor) = OUTPUT_MAP.get(category) {
        let description = processor.process(event.clone());

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

        let embed = build_event_embed(
            &output_config.color, &description, event.time, None
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