pub mod parse;
pub mod render;

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