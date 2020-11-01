use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::{char, not_line_ending},
    multi::many_till,
    sequence::preceded,
    IResult,
};

/// A comment is all characters between a non-escapted `%` percent sign and either a line break
/// sequence (LF, CRLF) or the end of the input (EOF).
fn comment(input: &str) -> IResult<&str, &str> {
    preceded(tag("%"), not_line_ending)(input)
}

/// Literal text.
fn text_span(input: &str) -> IResult<&str, &str> {
    let reserved = "\\{}%`'\r\n";

    take_while(move |c| !reserved.contains(c))(input)
}

/// Paragraphs collect smaller elements and can be terminated by:
///
/// - the `\par` macro
/// - a double line break sequence (LFLF, CRLFCRLF)
/// - the end of the input (EOF)
/// - TODO any vertical mode commands
///
/// FIXME remodel more closely to TeX hlist/vlist modes
fn paragraph(input: &str) -> IResult<&str, Vec<&str>> {
    let contents = alt((text_span, comment));
    let terminators = alt((tag(r"\par "), tag("\n\n"), tag("\r\n\r\n")));

    let (input, output) = many_till(contents, terminators)(input)?;
    Ok((input, output.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_test() {}

    mod comment {
        use super::*;

        #[test]
        fn parses_a_comment() {
            assert_eq!(comment("%comment\nrem"), Ok(("\nrem", "comment")));
            assert_eq!(comment("%comment\r\nrem"), Ok(("\r\nrem", "comment")));
        }

        #[test]
        fn parses_a_comment_terminated_by_eof() {
            assert_eq!(comment("%comment"), Ok(("", "comment")));
        }

        #[test]
        fn parses_an_empty_comment() {
            assert_eq!(comment("%"), Ok(("", "")));
        }
    }

    mod text_span {
        use super::*;

        #[test]
        fn parses_utf8_text() {
            assert_eq!(text_span(r"Introdução"), Ok((r"", "Introdução")));
        }

        #[test]
        fn stops_on_reserved_chars() {
            assert_eq!(text_span(r"text\rem"), Ok((r"\rem", "text")));
            assert_eq!(text_span(r"text%rem"), Ok((r"%rem", "text")));
            assert_eq!(text_span(r"text{rem"), Ok((r"{rem", "text")));
            assert_eq!(text_span(r"text}rem"), Ok((r"}rem", "text")));
            assert_eq!(text_span(r"text`rem"), Ok((r"`rem", "text")));
            assert_eq!(text_span(r"text'rem"), Ok((r"'rem", "text")));
            assert_eq!(text_span("text\nrem"), Ok(("\nrem", "text")));
            assert_eq!(text_span("text\r\nrem"), Ok(("\r\nrem", "text")));
        }
    }

    mod paragraph {
        use super::*;

        #[test]
        fn parses_ending_explicitly_with_par() {
            assert_eq!(paragraph(r"text\par rem"), Ok((r"rem", vec!["text"])));
        }

        #[test]
        fn parses_ending_implicitly_at_eof() {
            assert_eq!(paragraph(r"text"), Ok((r"", vec!["text"])));
        }

        #[test]
        fn parses_ending_implicitly_in_double_newline() {
            assert_eq!(paragraph("text\n\nrem"), Ok(("rem", vec!["text"])));
            assert_eq!(paragraph("text\r\n\r\nrem"), Ok(("rem", vec!["text"])));
        }

        #[test]
        fn parses_internal_comments() {
            assert_eq!(
                paragraph(r"text1%comment\ntext2\par rem"),
                Ok((r"rem", vec!["text1", "comment", "text2"]))
            );
        }
    }

    #[test]
    fn parses_isolated_samples() {
        assert!(comment("%!TEX root = ..Mercado 2014.tex\n").is_ok());
    }
}
