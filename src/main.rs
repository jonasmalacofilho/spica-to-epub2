use std::path::Path;

use spica_to_epub2::generator;
use spica_to_epub2::parser;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match &args[..] {
        [_, input_root] => {
            let book = parser::parse_book(Path::new(&input_root)).unwrap();
            eprintln!("{:#?}", book.ast);

            let bufs = generator::write_book(&book);
            bufs.iter()
                .enumerate()
                .for_each(|(i, x)| println!("<!-- file #{} -->\n\n{}", i, x));
        }
        _ => panic!("Usage: spica_to_epub2 <input_root>"),
    }
}
