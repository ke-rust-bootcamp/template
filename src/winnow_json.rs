use anyhow::{anyhow, Result};
use std::collections::HashMap;
use winnow::{
    ascii::{dec_int, float, multispace0},
    combinator::{alt, delimited, separated, separated_pair, trace},
    error::{ContextError, ErrMode, ParserError},
    prelude::*,
    stream::{AsChar, Stream, StreamIsPartial},
    token::take_until,
};

#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Int(i64),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

fn main() -> Result<()> {
    let s = r#"{
        "name": "John Doe",
        "age": 30,
        "is_student": false,
        "marks": [90.2, -80.3, 85.1],
        "address": {
            "city": "New York",
            "zip": 10001
        }
    }"#;

    let input = &mut (&*s);
    let v = parse_json(input)?;
    println!("{:#?}", v);
    Ok(())
}

fn parse_json(input: &str) -> Result<JsonValue> {
    let input = &mut (&*input);
    parse_value(input).map_err(|e: ErrMode<ContextError>| anyhow!("Failed to parse JSON: {:?}", e))
}

pub fn sep_with_space<Input, Output, Error, ParseNext>(
    mut parser: ParseNext,
) -> impl Parser<Input, (), Error>
where
    Input: Stream + StreamIsPartial + Clone + PartialEq,
    <Input as Stream>::Token: AsChar + Clone,
    Error: ParserError<Input>,
    ParseNext: Parser<Input, Output, Error>,
{
    trace("sep_with_space", move |input: &mut Input| {
        let _ = multispace0.parse_next(input)?;
        parser.parse_next(input)?;
        multispace0.parse_next(input)?;
        Ok(())
    })
}

fn parse_null(input: &mut &str) -> PResult<()> {
    "null".value(()).parse_next(input)
}

fn parse_bool(input: &mut &str) -> PResult<bool> {
    alt(("true", "false")).parse_to().parse_next(input)
}

// // FIXME: num parse doesn't work with scientific notation, fix it
// fn parse_num(input: &mut &str) -> PResult<Num> {
//     let sign = opt("-").map(|s| s.is_some()).parse_next(input)?;
//     let num = digit1.parse_to::<i64>().parse_next(input)?;
//     let ret: Result<(), ErrMode<ContextError>> = ".".value(()).parse_next(input);
//     if ret.is_ok() {
//         let frac = digit1.parse_to::<i64>().parse_next(input)?;
//         let v = format!("{}.{}", num, frac).parse::<f64>().unwrap();
//         Ok(if sign {
//             Num::Float(-v as _)
//         } else {
//             Num::Float(v as _)
//         })
//     } else {
//         Ok(if sign { Num::Int(-num) } else { Num::Int(num) })
//     }
// }

// json allows quoted strings to have escaped characters, we won't handle that here
fn parse_string(input: &mut &str) -> PResult<String> {
    let ret = delimited('"', take_until(0.., '"'), '"').parse_next(input)?;
    Ok(ret.to_string())
}

fn parse_array(input: &mut &str) -> PResult<Vec<JsonValue>> {
    let sep1 = sep_with_space('[');
    let sep2 = sep_with_space(']');
    let sep_comma = sep_with_space(',');
    let parse_values = separated(0.., parse_value, sep_comma);
    delimited(sep1, parse_values, sep2).parse_next(input)
}

fn parse_object(input: &mut &str) -> PResult<HashMap<String, JsonValue>> {
    let sep1 = sep_with_space('{');
    let sep2 = sep_with_space('}');
    let sep_comma = sep_with_space(',');
    let sep_colon = sep_with_space(':');

    let parse_kv_pair = separated_pair(parse_string, sep_colon, parse_value);
    let parse_kv = separated(.., parse_kv_pair, sep_comma);
    delimited(sep1, parse_kv, sep2).parse_next(input)
}

// 当前将 JSON 数字型全部解析为 float
fn parse_value(input: &mut &str) -> PResult<JsonValue> {
    alt((
        parse_null.value(JsonValue::Null),
        parse_bool.map(JsonValue::Bool),
        float.map(JsonValue::Number),
        dec_int.map(JsonValue::Int),
        parse_string.map(JsonValue::String),
        parse_array.map(JsonValue::Array),
        parse_object.map(JsonValue::Object),
    ))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_null() -> PResult<(), ContextError> {
        let input = "null";
        parse_null(&mut (&*input))?;

        Ok(())
    }

    #[test]
    fn test_parse_bool() -> PResult<(), ContextError> {
        let input = "true";
        let result = parse_bool(&mut (&*input))?;
        assert!(result);

        let input = "false";
        let result = parse_bool(&mut (&*input))?;
        assert!(!result);

        Ok(())
    }

    #[test]
    fn test_parse_num() -> PResult<(), ContextError> {
        let mut input = "123";
        let result: i64 = dec_int(&mut input)?;
        assert_eq!(result, 123);

        let input = "-123";
        let result = dec_int(&mut (&*input)).map(JsonValue::Int)?;
        assert_eq!(result, JsonValue::Int(-123));

        let input = "123.0";
        let result: f64 = float(&mut (&*input))?;
        assert_eq!(result, 123.0);

        let input = "-123.456";
        let result = parse_value(&mut (&*input))?;
        assert_eq!(result, JsonValue::Number(-123.456));

        Ok(())
    }

    #[test]
    fn test_parse_string() -> PResult<(), ContextError> {
        let input = r#""hello""#;
        let result = parse_string(&mut (&*input))?;
        assert_eq!(result, "hello");

        Ok(())
    }

    #[test]
    fn test_parse_array() -> PResult<(), ContextError> {
        let input = r#"[1, -2, 3]"#;
        let result = parse_array(&mut (&*input))?;

        assert_eq!(
            result,
            vec![
                JsonValue::Number(1.0),
                JsonValue::Number(-2.0),
                JsonValue::Number(3.0)
            ]
        );

        Ok(())
    }

    #[test]
    fn test_parse_object() -> PResult<(), ContextError> {
        let mut input = r#"{
        "a": "John Doe",
        "b": [1.0, -2.0]
    }"#;
        let result = parse_object(&mut input)?;
        let mut expected = HashMap::new();
        expected.insert("a".to_string(), JsonValue::String("John Doe".to_string()));
        expected.insert(
            "b".to_string(),
            JsonValue::Array(vec![JsonValue::Number(1.0), JsonValue::Number(-2.0)]),
        );
        println!("{:?}", expected);
        assert_eq!(result, expected);

        Ok(())
    }
}
