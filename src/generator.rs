use super::*;

pub fn write_book(book: &Book) -> Vec<String> {
    let mut ctx = Context::new();
    ctx.write(&book.ast);
    ctx.buffers
}

struct Context {
    buffers: Vec<String>,
    paragraph_count: u32,
    footnote_count: u32,
    is_paragraph: bool,
    can_start_paragraph: bool,
}

impl Context {
    fn new() -> Context {
        Context {
            buffers: vec![],
            paragraph_count: 0,
            footnote_count: 0,
            is_paragraph: false,
            can_start_paragraph: true,
        }
    }

    fn start_chapter(&mut self, title: &Atom) {
        self.buffers.push(String::new());
        self.close_paragraph_if_necessary();
        self.can_start_paragraph = false;
        self.write_raw("<h1>");
        self.write(title);
        self.write_raw("</h1>\n\n");
        self.can_start_paragraph = true;
    }

    fn start_section(&mut self, title: &Atom) {
        self.close_paragraph_if_necessary();
        self.can_start_paragraph = false;
        self.write_raw("<h2>");
        self.write(title);
        self.write_raw("</h2>\n\n");
        self.can_start_paragraph = true;
    }

    fn open_paragraph_if_necessary(&mut self) {
        if self.can_start_paragraph && !self.is_paragraph {
            let id = self.paragraph_count;
            self.paragraph_count += 1;
            self.write_raw(format!("<p id={}>", id).as_str());
            self.is_paragraph = true;
            self.can_start_paragraph = false;
        }
    }

    fn close_paragraph_if_necessary(&mut self) {
        if self.is_paragraph {
            self.write_raw("</p>\n\n");
            self.is_paragraph = false;
            self.can_start_paragraph = true;
        }
    }

    fn write_raw(&mut self, text: &str) {
        self.buffers.last_mut().unwrap().push_str(text);
    }

    fn write_text(&mut self, text: &str) {
        self.write_raw(text); // FIXME escape
    }

    fn write(&mut self, atom: &Atom) {
        match atom {
            Atom::List(list) => list.iter().for_each(|x| self.write(x)),
            Atom::Comment(_) | Atom::Ignore => {}
            Atom::Text(text) => {
                self.open_paragraph_if_necessary();
                self.write_text(text)
            }
            Atom::Escaped(character) => {
                self.open_paragraph_if_necessary();
                self.write_text(&character.to_string())
            }
            Atom::Special(symbol) | Atom::NamedSymbol(symbol) => {
                let text = match symbol.as_str() {
                    pass @ "\n" | pass @ "-" => pass,
                    "~" => "\u{00a0}",   // no-break space
                    "---" => "\u{2014}", // em-dash
                    "--" => "\u{2013}",  // en-dash
                    "``" => "\u{201c}",  // left double quotes
                    "''" => "\u{201d}",  // right double quotes
                    "`" => "\u{2018}",   // left single quote
                    "'" => "\u{2019}",   // right single quote
                    "textbackslash" => "\\",
                    "omission" => "[...]", // FIXME use better symbol
                    _ => todo!("Canot yet generate `{:?}`", atom),
                };
                self.open_paragraph_if_necessary();
                self.write_text(text);
            }
            Atom::StartChapter(title) => {
                self.start_chapter(title);
            }
            Atom::StartSection(title) => {
                self.start_section(title);
            }
            Atom::Footnote(contents) => {
                let id = self.footnote_count;
                self.footnote_count += 1;
                self.is_paragraph = false;
                // TODO
                self.is_paragraph = true;
            }
            Atom::Italic(contents) => {
                self.open_paragraph_if_necessary();
                self.is_paragraph = false;
                self.write_raw("<i>");
                self.write(contents);
                self.write_raw("</i>\n");
                self.is_paragraph = true;
            }
            Atom::BeginEnvironment(env) if env == "quotation" => {
                self.close_paragraph_if_necessary();
                // FIXME remove literal quotation marks from the manuscript
                self.write_raw("<q>\n\n");
            }
            Atom::EndEnvironment(env) if env == "quotation" => {
                self.close_paragraph_if_necessary();
                // FIXME remove literal quotation marks from the manuscript
                self.write_raw("</q>\n\n");
            }
            Atom::ParagraphEnd => self.close_paragraph_if_necessary(),
            Atom::BeginEnvironment(_) | Atom::EndEnvironment(_) => {
                todo!("Cannot yet generate `{:#?}`", atom)
            }
        }
    }
}
