lazy_static::lazy_static! {
    static ref LOWERCASE_WORDS: Vec<&'static str> = vec![
        "a","an","the","and","but","or","for","nor","on","at","to","in","of"
    ];
}

pub fn prettify_name(name: &str) -> String {
    let words: Vec<String> = name.replace("_", " ").split(" ").map(
        |word| {
            let mut copy = word.to_owned();
            if LOWERCASE_WORDS.contains(&copy.as_str()) { copy } else {
                if let Some(ch) = copy.get_mut(0..1) {
                    ch.make_ascii_uppercase();
                }
                copy
            }
        }
    ).collect();

    words.join(" ")
}

pub fn canonicalize_name(name: &str) -> String {
    name.replace(" ", "_").to_ascii_lowercase()
}

pub fn nation_link(nation: &str) -> String {
    format!("https://www.nationstates.net/nation={}", nation)
}

pub fn region_link(region: &str) -> String {
    format!("https://www.nationstates.net/region={}", region)
}

pub fn display_nation(name: &str, bold: bool) -> String {
    let display = prettify_name(name);
    let fmt = if bold { format!("**{}**", display) } else { display };
    format!("[{fmt}]({})", nation_link(name))
}

pub fn display_region(name: &str, bold: bool) -> String {
    let display = prettify_name(name);
    let fmt = if bold { format!("**{}**", display) } else { display };
    format!("[{fmt}]({})", region_link(name))
}

pub fn get_rmb_link(region: &str, postid: &str) -> String {
    format!(
        "https://www.nationstates.net/page=display_region_rmb/region={}?postid={}#p{}", 
        region, postid, postid
    )
}

pub fn display_rmb_post(region: &str, postid: &str) -> String {
    format!(
        "[{}]({})",
        postid,
        get_rmb_link(region, postid)
    )
}