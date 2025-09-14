use std::io::Write;

use base64::prelude::*;
use chrono::DateTime;
use uuid::Uuid;
use xml::{EventReader, EventWriter};

use crate::Uri;

use super::Llsd;

pub fn from_parser<R: std::io::Read>(parser: EventReader<R>) -> Result<Llsd, anyhow::Error> {
    use xml::reader::XmlEvent;
    let mut stack: Vec<Llsd> = Vec::new();
    let mut name_stack: Vec<String> = Vec::new();
    let mut key_stack: Vec<Option<String>> = Vec::new();
    let mut start = false;
    let mut end = false;

    for event in parser {
        match event {
            Ok(XmlEvent::StartElement { name, .. }) => {
                name_stack.push(name.local_name.clone());
                if !start {
                    if name.local_name.as_str() != "llsd" {
                        return Err(anyhow::anyhow!(
                            "Error parsing LLSD: expected <llsd> root element, got {}",
                            name.local_name
                        ));
                    }
                    start = true;
                    continue;
                }
                match name.local_name.as_str() {
                    "llsd" => {
                        return Err(anyhow::anyhow!(
                            "Error parsing LLSD: unexpected <llsd> element"
                        ));
                    }
                    "undef" => stack.push(Llsd::Undefined),
                    "boolean" => stack.push(Llsd::Boolean(false)),
                    "string" => stack.push(Llsd::String(String::new())),
                    "uuid" => stack.push(Llsd::Uuid(Default::default())),
                    "uri" => stack.push(Llsd::Uri(Uri::Empty)),
                    "date" => stack.push(Llsd::Date(Default::default())),
                    "binary" => stack.push(Llsd::Binary(Vec::new())),
                    "integer" => stack.push(Llsd::Integer(0)),
                    "real" => stack.push(Llsd::Real(0.0)),
                    "array" => stack.push(Llsd::Array(Vec::new())),
                    "map" => stack.push(Llsd::Map(Default::default())),
                    "key" => {
                        key_stack.push(None);
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Error parsing LLSD: unexpected element {}",
                            name.local_name
                        ));
                    }
                }
            }
            Ok(XmlEvent::Characters(data)) => {
                if key_stack.last() == Some(&None) {
                    key_stack.pop();
                    key_stack.push(Some(data.clone()));
                } else if let Some(llsd) = stack.last_mut() {
                    match llsd {
                        Llsd::Boolean(_) => match data.as_str() {
                            "true" => *llsd = Llsd::Boolean(true),
                            "false" => *llsd = Llsd::Boolean(false),
                            "1" => *llsd = Llsd::Boolean(true),
                            "0" => *llsd = Llsd::Boolean(false),
                            _ => {
                                return Err(anyhow::anyhow!(
                                    "Error parsing LLSD: expected boolean, got {}",
                                    data
                                ));
                            }
                        },
                        &mut Llsd::String(ref mut s) => s.push_str(data.as_str()),
                        &mut Llsd::Uuid(ref mut u) => *u = Uuid::parse_str(data.as_str())?,
                        &mut Llsd::Uri(ref mut u) => *u = Uri::parse(data.as_str()),
                        &mut Llsd::Date(ref mut d) => {
                            *d = DateTime::parse_from_rfc3339(data.as_str())?.into()
                        }
                        &mut Llsd::Binary(ref mut b) => {
                            *b = BASE64_STANDARD.decode(data.as_bytes())?
                        }
                        &mut Llsd::Integer(ref mut i) => *i = data.parse()?,
                        &mut Llsd::Real(ref mut r) => match data.as_str() {
                            "nan" => *r = f64::NAN,
                            "inf" => *r = f64::INFINITY,
                            "-inf" => *r = f64::NEG_INFINITY,
                            _ => *r = data.parse()?,
                        },
                        _ => {
                            return Err(anyhow::anyhow!(
                                "Error parsing LLSD: unexpected characters {}",
                                data
                            ));
                        }
                    }
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                if name_stack.pop().as_ref() != Some(&name.local_name) {
                    return Err(anyhow::anyhow!(
                        "Error parsing LLSD: unexpected end element {}",
                        name.local_name
                    ));
                }
                if name.local_name.as_str() == "key" {
                    if key_stack.last().is_none() {
                        return Err(anyhow::anyhow!("Error parsing LLSD: missing key"));
                    }
                } else if name.local_name.as_str() == "llsd" {
                    end = true;
                    break;
                } else if let Some(last) = stack.pop() {
                    match stack.last_mut() {
                        Some(Llsd::Array(parent)) => parent.push(last),
                        Some(Llsd::Map(parent)) => {
                            if let Some(Some(key)) = key_stack.pop() {
                                parent.insert(key.to_string(), last);
                            } else {
                                return Err(anyhow::anyhow!("Error parsing LLSD: missing key"));
                            }
                        }
                        _ => stack.push(last),
                    }
                } else {
                    return Err(anyhow::anyhow!(
                        "Error parsing LLSD: unexpected end element {}",
                        name.local_name
                    ));
                }
            }
            Err(e) => return Err(anyhow::anyhow!("Error parsing LLSD: {}", e)),
            _ => {}
        }
    }
    if !end {
        Err(anyhow::anyhow!(
            "Error parsing LLSD: unexpected end of input"
        ))
    } else if !key_stack.is_empty() {
        Err(anyhow::anyhow!("Error parsing LLSD: missing key"))
    } else if stack.len() > 1 {
        Err(anyhow::anyhow!(
            "Error parsing LLSD: expected 1 value, got {}",
            stack.len()
        ))
    } else {
        Ok(stack.pop().unwrap_or(Llsd::Undefined))
    }
}

pub fn from_str(data: &str) -> Result<Llsd, anyhow::Error> {
    from_parser(EventReader::from_str(data))
}

pub fn from_reader<R: std::io::Read>(reader: R) -> Result<Llsd, anyhow::Error> {
    from_parser(EventReader::new(reader))
}

pub fn from_slice(data: &[u8]) -> Result<Llsd, anyhow::Error> {
    from_parser(EventReader::new(std::io::Cursor::new(data)))
}

fn write_inner<W: Write>(llsd: &Llsd, w: &mut EventWriter<W>) -> Result<(), anyhow::Error> {
    use xml::writer::XmlEvent;
    let tag = |w: &mut EventWriter<W>, tag, text: &str| -> Result<(), anyhow::Error> {
        w.write(XmlEvent::start_element(tag))?;
        if !text.is_empty() {
            w.write(XmlEvent::characters(text))?;
        }
        w.write(XmlEvent::end_element())?;
        Ok(())
    };
    fn f64_to_xml(v: f64) -> String {
        let ss = v.to_string();
        if ss == "NaN" { "nan".to_string() } else { ss }
    }
    match llsd {
        Llsd::Undefined => tag(w, "undef", "")?,
        Llsd::Boolean(b) => tag(w, "boolean", if *b { "1" } else { "0" })?,
        Llsd::String(s) => tag(w, "string", s)?,
        Llsd::Uuid(u) => tag(w, "uuid", u.to_string().as_str())?,
        Llsd::Uri(u) => tag(w, "uri", u.as_str())?,
        Llsd::Date(d) => tag(w, "date", d.to_rfc3339().as_str())?,
        Llsd::Binary(b) => {
            if b.is_empty() {
                tag(w, "binary", "")?;
            } else {
                w.write(XmlEvent::start_element("binary").attr("encoding", "base64"))?;
                w.write(XmlEvent::characters(&BASE64_STANDARD.encode(b)))?;
                w.write(XmlEvent::end_element())?;
            }
        }
        Llsd::Integer(i) => tag(w, "integer", &i.to_string())?,
        Llsd::Real(r) => tag(w, "real", f64_to_xml(*r).as_str())?,
        Llsd::Array(a) => {
            w.write(XmlEvent::start_element("array"))?;
            for v in a {
                write_inner(v, w)?;
            }
            w.write(XmlEvent::end_element())?;
        }
        Llsd::Map(m) => {
            w.write(XmlEvent::start_element("map"))?;
            for (k, v) in m {
                tag(w, "key", k)?;
                write_inner(v, w)?;
            }
            w.write(XmlEvent::end_element())?;
        }
    }
    Ok(())
}

pub fn write<W: Write>(llsd: &Llsd, w: &mut EventWriter<W>) -> Result<(), anyhow::Error> {
    use xml::writer::XmlEvent;
    w.write(XmlEvent::start_element("llsd"))?;
    write_inner(llsd, w)?;
    w.write(XmlEvent::end_element())?;
    Ok(())
}

pub fn to_pretty_string(llsd: &Llsd) -> Result<String, anyhow::Error> {
    let mut buf = Vec::new();
    write(
        llsd,
        &mut EventWriter::new_with_config(
            &mut buf,
            xml::writer::EmitterConfig::new().perform_indent(true),
        ),
    )?;
    Ok(String::from_utf8(buf)?)
}

pub fn to_string(llsd: &Llsd) -> Result<String, anyhow::Error> {
    let mut buf = Vec::new();
    write(llsd, &mut EventWriter::new(&mut buf))?;
    Ok(String::from_utf8(buf)?)
}

pub fn to_request(llsd: &Llsd) -> Result<Vec<u8>, anyhow::Error> {
    let mut buf = Vec::new();
    write(
        llsd,
        &mut EventWriter::new_with_config(
            &mut buf,
            xml::writer::EmitterConfig::new().write_document_declaration(false),
        ),
    )?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::collections::HashMap;
    use url::Url;

    fn round_trip(llsd: Llsd) {
        let encoded = to_string(&llsd).expect("Failed to encode");
        let decoded = from_str(&encoded).expect("Failed to decode");
        assert_eq!(llsd, decoded);
    }

    #[test]
    fn undefined() {
        round_trip(Llsd::Undefined);
    }

    #[test]
    fn boolean() {
        round_trip(Llsd::Boolean(true));
        round_trip(Llsd::Boolean(false));
    }

    #[test]
    fn integer() {
        round_trip(Llsd::Integer(42));
    }

    #[test]
    fn real() {
        round_trip(Llsd::Real(13.1415));
    }

    #[test]
    fn string() {
        round_trip(Llsd::String("Hello, LLSD!".to_owned()));
    }

    #[test]
    fn uri() {
        let url = Url::parse("https://example.com/").unwrap();
        round_trip(Llsd::Uri(url.into()));
    }

    #[test]
    fn uuid() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        round_trip(Llsd::Uuid(uuid));
    }

    #[test]
    fn date() {
        let dt = Utc.timestamp_opt(1_620_000_000, 0).unwrap();
        round_trip(Llsd::Date(dt));
    }

    #[test]
    fn binary() {
        round_trip(Llsd::Binary(vec![0xde, 0xad, 0xbe, 0xef]));
    }

    #[test]
    fn array() {
        let arr = vec![
            Llsd::Integer(1),
            Llsd::String("two".into()),
            Llsd::Boolean(false),
        ];
        round_trip(Llsd::Array(arr));
    }

    #[test]
    fn map() {
        let mut map = HashMap::new();
        map.insert("answer".into(), Llsd::Integer(42));
        map.insert("pi".into(), Llsd::Real(13.14));
        map.insert("greeting".into(), Llsd::String("hello".into()));
        round_trip(Llsd::Map(map));
    }
}
