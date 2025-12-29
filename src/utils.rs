use caramel::ns::format::{nation_link, region_link, prettify_name};
use html_escape::decode_html_entities;

pub fn display_nation(name: &str, bold: bool) -> String {
    let display = prettify_name(name);
    let fmt = if bold { format!("**{display}**") } else { display };
    format!("[{fmt}]({})", nation_link(name))
}

pub fn display_region(name: &str, bold: bool) -> String {
    let display = prettify_name(name);
    let fmt = if bold { format!("**{display}**") } else { display };
    format!("[{fmt}]({})", region_link(name))
}

pub fn chamber_link(chamber: &str) -> &'static str {
    match chamber {
        "General Assembly" => "https://www.nationstates.net/page=ga",
        "Security Council" => "https://www.nationstates.net/page=sc",
        _ => "https://www.nationstates.net/page=un"
    }
}

pub fn display_chamber(chamber: &str, bold: bool) -> String {
    let fmt = if bold { format!("**{chamber}**") } else { chamber.to_string() };
    format!("[{fmt}]({})", chamber_link(chamber))
}

pub fn display_proposal_name(name: &str) -> String {
    format!("`{}`", decode_html_entities(name))
}

pub fn display_proposal_url(name: &str, chamber: &str, id: &str, bold: bool) -> String {
    let council = match chamber {
        "General Assembly" => "1",
        "Security Council" => "2",
        _ => "0"
    };

    let fmt = format!(
        "[{}](https://www.nationstates.net/page=WA_past_resolution/id={}/council={})", 
        decode_html_entities(name), id, council
    );
    
    if bold { format!("**{fmt}**") } else { fmt }
}