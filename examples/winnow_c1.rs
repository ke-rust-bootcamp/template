use winnow::error::ErrMode;
use winnow::error::ErrorKind;
use winnow::error::ParserError;
use winnow::stream::Stream;
use winnow::PResult;

fn parse_prefix(input: &mut &str) -> PResult<char> {
    let c = input
        .next_token()
        .ok_or_else(|| ErrMode::from_error_kind(input, ErrorKind::Token))?;
    if c != '0' {
        return Err(ErrMode::from_error_kind(input, ErrorKind::Verify));
    }
    Ok(c)
}

fn main() {
    let mut input = "0x1a2b Hello";

    let output = parse_prefix(&mut input).unwrap();

    assert_eq!(input, "x1a2b Hello");
    assert_eq!(output, '0');

    assert!(parse_prefix(&mut "d").is_err());
}
