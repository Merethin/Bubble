use caramel::ns::format::prettify_name;

use crate::utils::{display_nation, display_region};
use crate::nscode::Tag;

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

pub fn render_tags(tags: Vec<Tag<'_>>, limit: usize) -> String {
    let chars = render_as_bytes(tags, limit);

    String::from_utf8_lossy(&chars).to_string()
}