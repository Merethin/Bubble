use pest::{Parser, iterators::{Pairs, Pair}};
use pest_derive::Parser;
use bbx::BBParser;

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

#[derive(Parser)]
#[grammar = "nscode.pest"]
struct NsCodeParser;

fn walk_pair(pair: Pair<'_, Rule>) -> Vec<Tag<'_>> {
    match pair.as_rule() {
        Rule::tree => {
            walk_pairs(pair.into_inner())
        }

        Rule::TEXT | Rule::invalid_tag  => {
            vec![Tag::Text(pair.as_str())]
        }

        Rule::bold_tag => vec![Tag::Bold(walk_pairs(pair.into_inner()))],
        Rule::italic_tag => vec![Tag::Italic(walk_pairs(pair.into_inner()))],
        Rule::underline_tag => vec![Tag::Underline(walk_pairs(pair.into_inner()))],
        Rule::strike_tag => vec![Tag::Strike(walk_pairs(pair.into_inner()))],
        Rule::sub_tag => vec![Tag::Sub(walk_pairs(pair.into_inner()))],
        Rule::sup_tag => vec![Tag::Sup(walk_pairs(pair.into_inner()))],

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
            let children = walk_pairs(inner);
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
            let children = walk_pairs(inner);
            vec![Tag::Resolution((council, number, children))]
        }

        Rule::default_spoiler_tag => {
            let children = walk_pairs(pair.into_inner());
            vec![Tag::Spoiler((None, children))]
        }

        Rule::spoiler_tag => {
            let mut inner = pair.into_inner();
            let title = inner.next()
                .and_then(|p| if p.as_rule() == Rule::TEXT_INSIDE_TAG { Some(p.as_str()) } else { None });
            let children = walk_pairs(inner);
            vec![Tag::Spoiler((title, children))]
        }

        Rule::url_tag => {
            let mut inner = pair.into_inner();
            let href = inner.next()
                .and_then(|p| if p.as_rule() == Rule::TEXT_INSIDE_TAG { Some(p.as_str()) } else { None })
                .unwrap_or("");
            let children = walk_pairs(inner);
            vec![Tag::Url((href, children))]
        }

        Rule::pre_tag => {
            let children = walk_pairs(pair.into_inner());
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
            let children = walk_pairs(inner);
            vec![Tag::Quote((name, postid, children))]
        }

        _ => vec![],
    }
}

fn walk_pairs(pairs: Pairs<'_, Rule>) -> Vec<Tag<'_>> {
    pairs.flat_map(walk_pair).collect()
}

pub fn parse(text: &str) -> Option<Vec<Tag<'_>>> {
    if let Ok(pairs) = NsCodeParser::parse(Rule::tree, text) {
        Some(walk_pairs(pairs))
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