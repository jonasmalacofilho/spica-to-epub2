#[macro_use]
extern crate pest_derive;

#[derive(Debug, PartialEq)]
pub struct Argument {
    pub optional: bool,
    pub value: Atom,
}

#[derive(Debug, PartialEq)]
pub enum Atom {
    Comment(String),
    Text(String),
    Escaped(char),
    Special(String),
    Group(Box<Atom>),
    BracketGroup(Box<Atom>),
    ParagraphEnd,
    Command { name: String, args: Vec<Argument> },
    List(Vec<Atom>),
}

#[derive(Debug, PartialEq)]
pub struct Book {
    pub ast: Atom,
}

pub mod parser {
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
                },
                Rule::group => Atom::Group(Box::new(self.parse_atom_list(pair))),
                Rule::bracket_group => Atom::BracketGroup(Box::new(self.parse_atom_list(pair))),
                Rule::command => {
                    let mut inner = pair.into_inner();
                    let name = inner.next().unwrap().as_str().to_string();
                    if name == "include" {
                        let rel_name = inner.next().unwrap();
                        let rel_name: String = rel_name
                            .into_inner()
                            .map(|x| x.as_str().to_string())
                            .collect();
                        let filename = self.filename.with_file_name(rel_name + ".tex");
                        parse_file(filename.as_path()).unwrap() // FIXME panics
                    } else {
                        let args: Vec<Argument> = inner.map(|x| self.parse_argument(x)).collect();
                        // println!("{}, {} args", &name, args.len());
                        println!("{}", &name);
                        Atom::Command { name, args }
                    }
                }
                Rule::implicit_par => Atom::Command {
                    name: String::from("par"),
                    args: vec![],
                },
                _ => todo!("{:?}: {}", pair.as_rule(), pair.as_str()),
            }
        }

        fn parse_atom_list(&self, pair: Pair<Rule>) -> Atom {
            let mut list: Vec<Atom> = pair.into_inner().map(|x| self.parse_atom(x)).collect();
            if list.len() == 1 {
                list.pop().unwrap()
            } else {
                Atom::List(list)
            }
        }

        fn parse_argument(&self, pair: Pair<Rule>) -> Argument {
            let optional = match pair.as_rule() {
                Rule::group => false,
                Rule::bracket_group => true,
                _ => {
                    let filename = self.filename.to_string_lossy();
                    let (line, col) = pair.as_span().start_pos().line_col();
                    unreachable!("Unexpected {:?} at {}:{}:{}", pair.as_rule(), filename, line, col);
                }
            };
            let value = self.parse_atom_list(pair);
            Argument { optional, value }
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
                Atom::Command {
                    name: String::from("textbackslash"),
                    args: vec![]
                }
            );

            let filename = gen_test_file("\\somename[opt1]{req1}{req2}[opt2]");
            assert_eq!(
                parse_file(&filename).unwrap(),
                Atom::Command {
                    name: String::from("somename"),
                    args: vec![
                        Argument {
                            optional: true,
                            value: Atom::Text(String::from("opt1"))
                        },
                        Argument {
                            optional: false,
                            value: Atom::Text(String::from("req1"))
                        },
                        Argument {
                            optional: false,
                            value: Atom::Text(String::from("req2"))
                        },
                        Argument {
                            optional: true,
                            value: Atom::Text(String::from("opt2"))
                        }
                    ]
                }
            );

            let filename = gen_test_file("\\somename{Hello~World}");
            assert_eq!(
                parse_file(&filename).unwrap(),
                Atom::Command {
                    name: String::from("somename"),
                    args: vec![Argument {
                        optional: false,
                        value: Atom::List(vec![
                            Atom::Text(String::from("Hello")),
                            Atom::Special(String::from("~")),
                            Atom::Text(String::from("World"))
                        ])
                    },]
                }
            );

            let filename = gen_test_file("\\cmd1[\\cmd2{\\cmd3[Hello]}]");
            assert_eq!(
                parse_file(&filename).unwrap(),
                Atom::Command {
                    name: String::from("cmd1"),
                    args: vec![Argument {
                        optional: true,
                        value: Atom::Command {
                            name: String::from("cmd2"),
                            args: vec![Argument {
                                optional: false,
                                value: Atom::Command {
                                    name: String::from("cmd3"),
                                    args: vec![Argument {
                                        optional: true,
                                        value: Atom::Text(String::from("Hello"))
                                    }]
                                }
                            }]
                        }
                    }]
                }
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
                    Atom::Command {
                        name: String::from("par"),
                        args: vec![]
                    },
                    Atom::Text(String::from("World!"))
                ])
            );
        }
    }
}
