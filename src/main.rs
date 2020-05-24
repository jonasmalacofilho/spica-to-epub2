use spica_to_epub2::parser;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match &args[..] {
        [_, input_root] => {
            let book = parser::parse_book(Path::new(&input_root)).unwrap();
            println!("{:#?}", book.ast);
        }
        _ => panic!("Usage: spica_to_epub2 <input_root>"),
    }
}
