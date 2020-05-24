use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;
use std::fs;
use std::path::Path;

use super::*;

#[derive(Parser)]
#[grammar = "spica.pest"]
struct SpicaParser;

pub fn parse_book(root_filename: &Path) -> Result<Book, Error<Rule>> {
    Ok(Book {
        ast: parse_file(root_filename)?,
    })
}

fn parse_file(filename: &Path) -> Result<Atom, Error<Rule>> {
    // dbg!(filename);
    let ctx = Context::new(filename);
    let input = fs::read_to_string(&filename).unwrap(); // FIXME panics
    let raw_file = SpicaParser::parse(Rule::raw_file, &input)?.next().unwrap();
    Ok(ctx.parse_atom(raw_file.into_inner().next().unwrap()))
}

struct Context<'a> {
    filename: &'a Path,
}

impl<'a> Context<'a> {
    pub fn new(filename: &Path) -> Context {
        Context { filename }
    }

    pub fn parse_atom(&self, pair: Pair<Rule>) -> Atom {
        match pair.as_rule() {
            Rule::file => self.parse_atom_list(pair),
            Rule::COMMENT => Atom::Comment(pair.as_str().trim().to_string()),
            Rule::include => {
                let rel_name = pair.into_inner().next().unwrap();
                let rel_name = self.parse_raw_argument(rel_name);
                let filename = self.filename.with_file_name(rel_name + ".tex");
                parse_file(filename.as_path()).unwrap() // FIXME panics
            }
            Rule::text => Atom::Text(pair.as_str().to_string()),
            Rule::escape_sequence => Atom::Escaped(pair.as_str().chars().nth(1).unwrap()),
            Rule::special_text => {
                // Atom::Special(match pair.as_str() {
                //     "~" => '\u{00a0}'.to_string(),   // no-break space
                //     "---" => '\u{2014}'.to_string(), // em-dash
                //     "--" => '\u{2013}'.to_string(),  // en-dash
                //     "``" => '\u{201c}'.to_string(),  // left double quotes
                //     "''" => '\u{201d}'.to_string(),  // right double quotes
                //     "`" => '\u{2018}'.to_string(),   // left single quote
                //     "'" => '\u{2019}'.to_string(),   // right single quote
                //     other => other.to_string(),
                // })
                Atom::Special(pair.as_str().to_string())
            }
            Rule::textbackslash | Rule::omission => Atom::NamedSymbol(pair.as_str().to_string()),
            Rule::chapter => self.parse_simple_command(pair, Atom::StartChapter),
            Rule::footnote => self.parse_simple_command(pair, Atom::Footnote),
            Rule::manuscriptit => self.parse_simple_command(pair, Atom::Italic),
            Rule::mbox => {
                let contents = pair.into_inner().next().unwrap();
                self.parse_atom_list(contents)
            }
            Rule::begin => {
                let name = pair.into_inner().next().unwrap();
                Atom::BeginEnvironment(self.parse_raw_argument(name))
            }
            Rule::end => {
                let name = pair.into_inner().next().unwrap();
                Atom::EndEnvironment(self.parse_raw_argument(name))
            }
            Rule::implicit_par => Atom::ParagraphEnd,
            Rule::sloppy | Rule::cbreak | Rule::clearpage | Rule::fussy => Atom::Ignore,
            _ => todo!("{:?}: {}", pair.as_rule(), pair.as_str()),
        }
    }

    fn parse_simple_command(&self, pair: Pair<Rule>, node: fn(Box<Atom>) -> Atom) -> Atom {
        let contents = pair.into_inner().next().unwrap();
        node(Box::new(self.parse_atom_list(contents)))
    }

    fn parse_atom_list(&self, pair: Pair<Rule>) -> Atom {
        let mut list: Vec<Atom> = pair.into_inner().map(|x| self.parse_atom(x)).collect();
        if list.len() == 1 {
            list.pop().unwrap()
        } else {
            Atom::List(list)
        }
    }

    fn parse_raw_argument(&self, pair: Pair<Rule>) -> String {
        pair.into_inner().map(|x| x.as_str().to_string()).collect()
    }
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    fn gen_test_file(contents: &str) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut h = DefaultHasher::new();
        contents.hash(&mut h);

        let mut path = std::env::temp_dir();
        path.push(module_path!().replace("::", "/"));
        path.push(h.finish().to_string());

        if let Ok(meta) = fs::metadata(&path) {
            if meta.is_file() && meta.len() == contents.len() as u64 {
                return path;
            }
        }

        let mut tmp = path.clone();
        tmp.set_extension("tmp");
        fs::create_dir_all(&tmp.parent().unwrap()).unwrap();
        fs::write(&tmp, contents).unwrap();
        fs::rename(&tmp, &path).unwrap();
        path
    }

    #[test]
    fn test_file_generation() {
        let test_string = "Hello, \\emph{World}!";
        let filename = gen_test_file(test_string);
        assert!(filename.is_file());
        assert_eq!(fs::read_to_string(&filename).unwrap(), test_string);
    }

    #[test]
    fn empty() {
        let filename = gen_test_file("");
        assert_eq!(parse_file(&filename).unwrap(), Atom::List(vec![]));
    }

    #[test]
    fn comment() {
        let filename = gen_test_file("% Comment 1\nHello, World! % Comment 2\n% Comment 3");
        assert_eq!(
            parse_file(&filename).unwrap(),
            Atom::List(vec![
                Atom::Comment(String::from("% Comment 1")),
                Atom::Text(String::from("Hello, World! ")),
                Atom::Comment(String::from("% Comment 2")),
                Atom::Comment(String::from("% Comment 3"))
            ])
        );
    }

    #[test]
    fn simple_text() {
        let filename = gen_test_file("Hello, World!");
        assert_eq!(
            parse_file(&filename).unwrap(),
            Atom::Text(String::from("Hello, World!"))
        );
    }

    #[test]
    fn escape_sequence() {
        let filename = gen_test_file("\\&\\%\\$\\#\\_\\{\\}");
        assert_eq!(
            parse_file(&filename).unwrap(),
            Atom::List(vec![
                Atom::Escaped('&'),
                Atom::Escaped('%'),
                Atom::Escaped('$'),
                Atom::Escaped('#'),
                Atom::Escaped('_'),
                Atom::Escaped('{'),
                Atom::Escaped('}')
            ])
        );
    }

    #[test]
    fn special_text() {
        let filename = gen_test_file("~-----``''`'");
        assert_eq!(
            parse_file(&filename).unwrap(),
            Atom::List(vec![
                Atom::Special(String::from("~")),
                Atom::Special(String::from("---")),
                Atom::Special(String::from("--")),
                Atom::Special(String::from("``")),
                Atom::Special(String::from("''")),
                Atom::Special(String::from("`")),
                Atom::Special(String::from("'"))
            ])
        );
    }

    #[test]
    fn commands() {
        let filename = gen_test_file("\\textbackslash");
        assert_eq!(
            parse_file(&filename).unwrap(),
            Atom::NamedSymbol(String::from("textbackslash"))
        );

        let filename = gen_test_file("\\chapter{Introduction}");
        assert_eq!(
            parse_file(&filename).unwrap(),
            Atom::StartChapter(Box::new(Atom::Text(String::from("Introduction"))))
        );
    }

    #[test]
    fn newlines() {
        let filename = gen_test_file("Hello\nWorld!");
        assert_eq!(
            parse_file(&filename).unwrap(),
            Atom::List(vec![
                Atom::Text(String::from("Hello")),
                Atom::Special(String::from("\n")),
                Atom::Text(String::from("World!"))
            ])
        );

        let filename = gen_test_file("Hello\n\nWorld!");
        assert_eq!(
            parse_file(&filename).unwrap(),
            Atom::List(vec![
                Atom::Text(String::from("Hello")),
                Atom::ParagraphEnd,
                Atom::Text(String::from("World!"))
            ])
        );
    }
}
