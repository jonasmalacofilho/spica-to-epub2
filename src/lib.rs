#[macro_use]
extern crate pest_derive;

#[derive(Debug, PartialEq)]
pub enum Atom {
    List(Vec<Atom>),
    Comment(String),
    Text(String),
    Escaped(char),
    Special(String),
    NamedSymbol(String),
    StartChapter(Box<Atom>),
    Footnote(Box<Atom>),
    Italic(Box<Atom>),
    BeginEnvironment(String),
    EndEnvironment(String),
    ParagraphEnd,
    Ignore,
}

#[derive(Debug, PartialEq)]
pub struct Book {
    pub ast: Atom,
}

pub mod parser;
