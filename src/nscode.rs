use pest::{Parser, iterators::{Pairs, Pair}};
use pest_derive::Parser;
use bbx::BBParser;

use caramel::ns::format::prettify_name;

use crate::utils::{display_nation, display_region};

pub enum Tag<'a> {
    Text(&'a str),
    Bold(Vec<Tag<'a>>),
    Italic(Vec<Tag<'a>>),
    Underline(Vec<Tag<'a>>),
    Strike(Vec<Tag<'a>>),
    Sub(Vec<Tag<'a>>),
    Sup(Vec<Tag<'a>>),
    Nation(&'a str),
    Region(&'a str),
    Proposal((&'a str, Vec<Tag<'a>>)),
    Resolution((&'a str, &'a str, Vec<Tag<'a>>)),
    Url((&'a str, Vec<Tag<'a>>)),
    Pre(Vec<Tag<'a>>),
    Quote((&'a str, &'a str, Vec<Tag<'a>>)),
    Spoiler((Option<&'a str>, Vec<Tag<'a>>)),
}

fn add_until_limit(vec: &mut Vec<u8>, bytes: &[u8], limit: &mut usize) {
    if bytes.len() > *limit {
        vec.extend_from_slice(&bytes[0..*limit]);
        *limit = 0;
    } else {
        vec.extend_from_slice(bytes);
        *limit -= bytes.len();
    }
}

fn add_if_within_limit(vec: &mut Vec<u8>, bytes: &[u8], limit: &mut usize) -> bool {
    if bytes.len() > *limit {
        false
    } else {
        vec.extend_from_slice(bytes);
        *limit -= bytes.len();
        true
    }
}

fn wrap_if_within_limit(vec: &mut Vec<u8>, bytes: &[u8], start: &[u8], end: &[u8], limit: &mut usize) {
    if bytes.len() <= *limit {
        if bytes.len() + start.len() + end.len() > *limit {
            vec.extend_from_slice(bytes);
            *limit -= bytes.len();
        } else {
            vec.extend_from_slice(start);
            *limit -= start.len();
            vec.extend_from_slice(bytes);
            *limit -= bytes.len();
            vec.extend_from_slice(end);
            *limit -= end.len();
        }
    }
}

fn wrap_lines(vec: &mut Vec<u8>, bytes: &[u8], start: &[u8], end: &[u8], limit: &mut usize) {
    for slice in bytes.split_inclusive(|&v| v == b'\n') {
        if slice.ends_with(&[b'\n']) {
            wrap_if_within_limit(
                vec, &slice[0..slice.len()-1], start, end, limit
            );
            add_if_within_limit(vec, &[b'\n'], limit);
        } else {
            wrap_if_within_limit(
                vec, slice, start, end, limit
            );
        }
    }
}

pub fn render_as_bytes(tags: Vec<Tag<'_>>, mut limit: usize) -> Vec<u8> {
    let mut chars: Vec<u8> = Vec::new();

    if limit == 0 { return chars; }

    for tag in tags {
        match tag {
            Tag::Text(text) => {
                add_until_limit(&mut chars, text.as_bytes(), &mut limit);
            }
            Tag::Bold(inner_tags) => {
                let bytes = render_as_bytes(inner_tags, limit.saturating_sub(4));
                wrap_lines(&mut chars, &bytes, "**".as_bytes(), "**".as_bytes(), &mut limit);
            },
            Tag::Italic(inner_tags) => {
                let bytes = render_as_bytes(inner_tags, limit.saturating_sub(2));
                wrap_lines(&mut chars, &bytes, "*".as_bytes(), "*".as_bytes(), &mut limit);
            },
            Tag::Underline(inner_tags) => {
                let bytes = render_as_bytes(inner_tags, limit.saturating_sub(4));
                wrap_lines(&mut chars, &bytes, "__".as_bytes(), "__".as_bytes(), &mut limit);
            },
            Tag::Strike(inner_tags) => {
                let bytes = render_as_bytes(inner_tags, limit.saturating_sub(4));
                wrap_lines(&mut chars, &bytes, "~~".as_bytes(), "~~".as_bytes(), &mut limit);
            },
            Tag::Sub(inner_tags) | Tag::Sup(inner_tags) | Tag::Spoiler((_, inner_tags)) => {
                let bytes = render_as_bytes(inner_tags, limit);
                add_until_limit(&mut chars, &bytes, &mut limit);
            },
            Tag::Nation(name) => {
                let string = display_nation(name, false);
                add_if_within_limit(&mut chars, string.as_bytes(), &mut limit);
            }
            Tag::Region(name) => {
                let string = display_region(name, false);
                add_if_within_limit(&mut chars, string.as_bytes(), &mut limit);
            }
            Tag::Proposal((id, inner_tags)) => {
                let url = format!("https://www.nationstates.net/page=UN_view_proposal/id={id}");

                let bytes = render_as_bytes(inner_tags, limit);
                wrap_lines(
                    &mut chars, &bytes, 
                    "[".as_bytes(), 
                    format!("]({url})").as_bytes(),
                    &mut limit
                );
            }
            Tag::Resolution((chamber, id, inner_tags)) => {
                let url = match chamber {
                    "UN" => format!("https://www.nationstates.net/page=WA_past_resolution/id={id}/un=1"),
                    "GA" => format!("https://www.nationstates.net/page=WA_past_resolution/id={id}/council=1"),
                    "SC" => format!("https://www.nationstates.net/page=WA_past_resolution/id={id}/council=2"),
                    _ => String::new(),
                };

                let bytes = render_as_bytes(inner_tags, limit);
                wrap_lines(
                    &mut chars, &bytes,
                    "[".as_bytes(), 
                    format!("]({url})").as_bytes(),
                    &mut limit
                );
            },
            Tag::Url((url, inner_tags)) => {
                let bytes = render_as_bytes(inner_tags, limit);
                wrap_lines(
                    &mut chars, &bytes, 
                    "[".as_bytes(), 
                    format!("]({url})").as_bytes(),
                    &mut limit
                );
            },
            Tag::Pre(inner_tags) => {
                let bytes = render_as_bytes(inner_tags, limit.saturating_sub(2));
                wrap_lines(&mut chars, &bytes, "`".as_bytes(), "`".as_bytes(), &mut limit);
            },
            Tag::Quote((nation, id, inner_tags)) => {
                let url = format!("https://www.nationstates.net/page=rmb/postid={id}");

                if let Some(char) = chars.last() && *char != b'\n' {
                    add_if_within_limit(&mut chars, &[b'\n'], &mut limit);
                }

                let bytes = render_as_bytes(inner_tags, limit.min(512));
                if nation != "0" && id != "0" {
                    add_if_within_limit(
                        &mut chars, 
                        format!("[Quoted from {}:]({})\n", prettify_name(nation), url).as_bytes(), 
                        &mut limit
                    );
                }

                wrap_lines(&mut chars, &bytes, "> ".as_bytes(), "".as_bytes(), &mut limit);
            }
        }
    }

    chars
}

pub fn render(tags: Vec<Tag<'_>>, limit: usize) -> String {
    let chars = render_as_bytes(tags, limit);

    String::from_utf8_lossy(&chars).to_string()
}

#[derive(Parser)]
#[grammar = "nscode.pest"]
struct NsCodeParser;

fn walk_pair(pair: Pair<'_, Rule>) -> Vec<Tag<'_>> {
    match pair.as_rule() {
        Rule::tree => {
            walk(pair.into_inner())
        }

        Rule::TEXT | Rule::invalid_tag  => {
            vec![Tag::Text(pair.as_str())]
        }

        Rule::bold_tag => vec![Tag::Bold(walk(pair.into_inner()))],
        Rule::italic_tag => vec![Tag::Italic(walk(pair.into_inner()))],
        Rule::underline_tag => vec![Tag::Underline(walk(pair.into_inner()))],
        Rule::strike_tag => vec![Tag::Strike(walk(pair.into_inner()))],
        Rule::sub_tag => vec![Tag::Sub(walk(pair.into_inner()))],
        Rule::sup_tag => vec![Tag::Sup(walk(pair.into_inner()))],

        Rule::nation_tag => {
            let name = pair.into_inner()
                .find(|p| p.as_rule() == Rule::NAME)
                .map_or("", |p| p.as_str());
            vec![Tag::Nation(name)]
        }

        Rule::region_tag => {
            let name = pair.into_inner()
                .find(|p| p.as_rule() == Rule::NAME)
                .map_or("", |p| p.as_str());
            vec![Tag::Region(name)]
        }

        Rule::proposal_tag => {
            let mut inner = pair.into_inner();
            let id = inner.next()
                .and_then(|p| if p.as_rule() == Rule::PROPOSAL { Some(p.as_str()) } else { None })
                .unwrap_or("");
            let children = walk(inner);
            vec![Tag::Proposal((id, children))]
        }

        Rule::resolution_tag => {
            let mut inner = pair.into_inner();
            let council = inner.next()
                .and_then(|p| if p.as_rule() == Rule::council { Some(p.as_str()) } else { None })
                .unwrap_or("");
            let number = inner.next()
                .and_then(|p| if p.as_rule() == Rule::NUMBER { Some(p.as_str()) } else { None })
                .unwrap_or("");
            let children = walk(inner);
            vec![Tag::Resolution((council, number, children))]
        }

        Rule::default_spoiler_tag => {
            let children = walk(pair.into_inner());
            vec![Tag::Spoiler((None, children))]
        }

        Rule::spoiler_tag => {
            let mut inner = pair.into_inner();
            let title = inner.next()
                .and_then(|p| if p.as_rule() == Rule::TEXT_INSIDE_TAG { Some(p.as_str()) } else { None });
            let children = walk(inner);
            vec![Tag::Spoiler((title, children))]
        }

        Rule::url_tag => {
            let mut inner = pair.into_inner();
            let href = inner.next()
                .and_then(|p| if p.as_rule() == Rule::TEXT_INSIDE_TAG { Some(p.as_str()) } else { None })
                .unwrap_or("");
            let children = walk(inner);
            vec![Tag::Url((href, children))]
        }

        Rule::pre_tag => {
            let children = walk(pair.into_inner());
            vec![Tag::Pre(children)]
        }

        Rule::quote_tag => {
            let mut inner = pair.into_inner();
            let name = inner.next()
                .and_then(|p| if p.as_rule() == Rule::NAME { Some(p.as_str()) } else { None })
                .unwrap_or("");
            let postid = inner.next()
                .and_then(|p| if p.as_rule() == Rule::NUMBER { Some(p.as_str()) } else { None })
                .unwrap_or("");
            let children = walk(inner);
            vec![Tag::Quote((name, postid, children))]
        }

        _ => vec![],
    }
}

fn walk(pairs: Pairs<'_, Rule>) -> Vec<Tag<'_>> {
    pairs.flat_map(walk_pair).collect()
}

pub fn parse(text: &str) -> Option<Vec<Tag<'_>>> {
    if let Ok(pairs) = NsCodeParser::parse(Rule::tree, text) {
        Some(walk(pairs))
    } else {
        None
    }
}

pub fn remove_subquotes(text: &str) -> String {
    let mut parser = BBParser::new(text);
    let mut result: Vec<String> = Vec::new();

    let mut quote_level: u64 = 0;

    while let Some(token) = parser.next() {
        if token.is_open("quote") {
            quote_level += 1;
        } else if token.is_close("quote") {
            quote_level = quote_level.saturating_sub(1);
        } else if quote_level == 0 {
            result.push(token.span.to_owned());
        }
    }

    result.into_iter().collect::<String>().trim().to_owned()
}