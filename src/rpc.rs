use base64::prelude::*;
use chrono::DateTime;
use xml::{EventReader, EventWriter};

use super::Llsd;

#[derive(Debug, Clone, PartialEq)]
pub enum XmlRpc {
    MethodCall(String, Llsd),
    MethodResponse(Llsd),
}

impl XmlRpc {
    pub fn new_method_call(method: String, llsd: Llsd) -> Self {
        XmlRpc::MethodCall(method, llsd)
    }

    pub fn new_method_response(llsd: Llsd) -> Self {
        XmlRpc::MethodResponse(llsd)
    }

    pub fn method(&self) -> Option<&str> {
        match self {
            XmlRpc::MethodCall(method, _) => Some(method),
            XmlRpc::MethodResponse(_) => None,
        }
    }

    pub fn llsd(&self) -> &Llsd {
        match self {
            XmlRpc::MethodCall(_, llsd) => llsd,
            XmlRpc::MethodResponse(llsd) => llsd,
        }
    }
}

impl AsRef<Llsd> for XmlRpc {
    fn as_ref(&self) -> &Llsd {
        self.llsd()
    }
}

impl AsMut<Llsd> for XmlRpc {
    fn as_mut(&mut self) -> &mut Llsd {
        match self {
            XmlRpc::MethodCall(_, llsd) => llsd,
            XmlRpc::MethodResponse(llsd) => llsd,
        }
    }
}

impl From<XmlRpc> for Llsd {
    fn from(rpc: XmlRpc) -> Self {
        match rpc {
            XmlRpc::MethodCall(_, llsd) => llsd,
            XmlRpc::MethodResponse(llsd) => llsd,
        }
    }
}

impl From<Llsd> for XmlRpc {
    fn from(llsd: Llsd) -> Self {
        XmlRpc::MethodResponse(llsd)
    }
}

impl From<(String, Llsd)> for XmlRpc {
    fn from((method, llsd): (String, Llsd)) -> Self {
        XmlRpc::MethodCall(method, llsd)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expected {
    None,
    Data,
    Member,
    Name,
    Value,
    XmlRpcHeader,
    MethodCallName,
    Parmas,
    Param,
}

pub fn from_parser<R: std::io::Read>(parser: EventReader<R>) -> Result<XmlRpc, anyhow::Error> {
    use xml::reader::XmlEvent;
    let mut stack: Vec<Llsd> = Vec::new();
    let mut name_stack: Vec<String> = Vec::new();
    let mut key_stack: Vec<String> = Vec::new();

    let mut expect_value = Expected::XmlRpcHeader;
    let mut method = None;

    for event in parser {
        match event {
            Ok(XmlEvent::StartElement { name, .. }) => {
                name_stack.push(name.local_name.clone());
                match (expect_value, name.local_name.as_str()) {
                    (Expected::Data, "data") => expect_value = Expected::Value,
                    (Expected::Member, "member") => expect_value = Expected::Name,
                    (Expected::Name, "name") => expect_value = Expected::Value,
                    (Expected::Value, "value") => expect_value = Expected::None,
                    (Expected::XmlRpcHeader, "methodResponse") => expect_value = Expected::Parmas,
                    (Expected::XmlRpcHeader, "methodCall") => {
                        expect_value = Expected::MethodCallName
                    }
                    (Expected::MethodCallName, "methodName") => expect_value = Expected::Parmas,
                    (Expected::Parmas, "params") => expect_value = Expected::Param,
                    (Expected::Param, "param") => expect_value = Expected::Value,
                    (Expected::None, "nil") => stack.push(Llsd::Undefined),
                    (Expected::None, "boolean") => stack.push(Llsd::Boolean(false)),
                    (Expected::None, "string") => stack.push(Llsd::String(String::new())),
                    (Expected::None, "int") => stack.push(Llsd::Integer(0)),
                    (Expected::None, "double") => stack.push(Llsd::Real(0.0)),
                    (Expected::None, "dateTime.iso8601") => {
                        stack.push(Llsd::Date(Default::default()))
                    }
                    (Expected::None, "base64") => stack.push(Llsd::Binary(Vec::new())),
                    (Expected::None, "array") => {
                        stack.push(Llsd::Array(Vec::new()));
                        expect_value = Expected::Data;
                    }
                    (Expected::None, "struct") => {
                        stack.push(Llsd::Map(Default::default()));
                        expect_value = Expected::Member;
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Error parsing XML-RPC: unexpected element {}",
                            name.local_name
                        ));
                    }
                }
            }
            Ok(XmlEvent::Characters(data)) => {
                let data = data.trim();
                if expect_value == Expected::MethodCallName {
                    method = Some(data.to_string());
                } else if name_stack.last().map(|s| s.as_str()) == Some("name") {
                    key_stack.push(data.to_string());
                } else if let Some(llsd) = stack.last_mut() {
                    match llsd {
                        Llsd::Boolean(_) => match data {
                            "true" => *llsd = Llsd::Boolean(true),
                            "false" => *llsd = Llsd::Boolean(false),
                            "1" => *llsd = Llsd::Boolean(true),
                            "0" => *llsd = Llsd::Boolean(false),
                            _ => {
                                return Err(anyhow::anyhow!(
                                    "Error parsing XML-RPC: expected boolean, got {}",
                                    data
                                ));
                            }
                        },
                        &mut Llsd::String(ref mut s) => s.push_str(data),
                        &mut Llsd::Date(ref mut d) => {
                            *d = DateTime::parse_from_rfc3339(data)?.into()
                        }
                        &mut Llsd::Binary(ref mut b) => {
                            *b = BASE64_STANDARD.decode(data.as_bytes())?
                        }
                        &mut Llsd::Integer(ref mut i) => *i = data.parse()?,
                        &mut Llsd::Real(ref mut r) => match data {
                            "nan" => *r = f64::NAN,
                            "inf" => *r = f64::INFINITY,
                            "-inf" => *r = f64::NEG_INFINITY,
                            _ => *r = data.parse()?,
                        },
                        _ => {
                            return Err(anyhow::anyhow!(
                                "Error parsing XML-RPC: unexpected characters {}",
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
                match name.local_name.as_str() {
                    "struct" | "array" if stack.len() > 1 => {
                        if let Some(parent) = stack.get(stack.len() - 2) {
                            if parent.is_array() {
                                expect_value = Expected::Value;
                            } else if parent.is_map() {
                                expect_value = Expected::Member;
                            } else {
                                return Err(anyhow::anyhow!(
                                    "Error parsing XML-RPC: not a map or array"
                                ));
                            }
                        }
                    }
                    "member" => {
                        let Some(key) = key_stack.pop() else {
                            return Err(anyhow::anyhow!("Error parsing XML-RPC: missing key"));
                        };
                        let Some(value) = stack.pop() else {
                            return Err(anyhow::anyhow!(
                                "Error parsing XML-RPC: unexpected end element {}",
                                name.local_name
                            ));
                        };
                        let Some(Llsd::Map(parent)) = stack.last_mut() else {
                            return Err(anyhow::anyhow!("Error parsing XML-RPC: not a map"));
                        };
                        parent.insert(key.to_string(), value);
                        expect_value = Expected::Member;
                    }
                    "value" if stack.len() > 1 => {
                        let Some(value) = stack.pop() else {
                            return Err(anyhow::anyhow!(
                                "Error parsing XML-RPC: unexpected end element {}",
                                name.local_name
                            ));
                        };
                        if let Some(Llsd::Array(parent)) = stack.last_mut() {
                            parent.push(value);
                            expect_value = Expected::Value;
                        } else {
                            stack.push(value);
                        }
                    }
                    _ => {}
                };
            }
            Err(e) => return Err(anyhow::anyhow!("Error parsing XML-RPC: {}", e)),
            _ => {}
        }
    }
    if let Some(llsd) = stack.pop() {
        if !stack.is_empty() {
            return Err(anyhow::anyhow!(
                "Error parsing XML-RPC: expected 1 value, got {}",
                stack.len() + 1
            ));
        }
        if let Some(method) = method {
            Ok(XmlRpc::MethodCall(method, llsd))
        } else {
            Ok(XmlRpc::MethodResponse(llsd))
        }
    } else {
        Err(anyhow::anyhow!("Error parsing XML-RPC: missing key"))
    }
}

pub fn from_str(data: &str) -> Result<XmlRpc, anyhow::Error> {
    from_parser(EventReader::from_str(data))
}

pub fn from_reader<R: std::io::Read>(reader: R) -> Result<XmlRpc, anyhow::Error> {
    from_parser(EventReader::new(reader))
}

pub fn from_slice(data: &[u8]) -> Result<XmlRpc, anyhow::Error> {
    from_parser(EventReader::new(std::io::Cursor::new(data)))
}

fn write_inner<W: std::io::Write>(
    llsd: &Llsd,
    w: &mut EventWriter<W>,
) -> Result<(), anyhow::Error> {
    use xml::writer::XmlEvent;
    let tag = |w: &mut EventWriter<W>, tag, text: &str| -> Result<(), anyhow::Error> {
        w.write(XmlEvent::start_element(tag))?;
        if !text.is_empty() {
            w.write(XmlEvent::characters(text))?;
        }
        w.write(XmlEvent::end_element())?;
        Ok(())
    };
    match llsd {
        Llsd::Undefined => tag(w, "nil", ""),
        Llsd::Boolean(b) => tag(w, "boolean", if *b { "1" } else { "0" }),
        Llsd::Integer(i) => tag(w, "int", &i.to_string()),
        Llsd::Real(r) => tag(w, "double", &r.to_string()),
        Llsd::String(s) => tag(w, "string", s),
        Llsd::Uri(u) => tag(w, "string", u.as_str()),
        Llsd::Uuid(u) => tag(w, "string", &u.to_string()),
        Llsd::Date(d) => tag(w, "dateTime.iso8601", &d.to_rfc3339()),
        Llsd::Binary(b) => tag(w, "base64", &BASE64_STANDARD.encode(b)),
        Llsd::Array(a) => {
            w.write(XmlEvent::start_element("array"))?;
            w.write(XmlEvent::start_element("data"))?;
            for llsd in a {
                w.write(XmlEvent::start_element("value"))?;
                write_inner(llsd, w)?;
                w.write(XmlEvent::end_element())?;
            }
            w.write(XmlEvent::end_element())?;
            w.write(XmlEvent::end_element())?;
            Ok(())
        }
        Llsd::Map(m) => {
            w.write(XmlEvent::start_element("struct"))?;
            for (k, v) in m {
                w.write(XmlEvent::start_element("member"))?;
                tag(w, "name", k)?;
                w.write(XmlEvent::start_element("value"))?;
                write_inner(v, w)?;
                w.write(XmlEvent::end_element())?;
                w.write(XmlEvent::end_element())?;
            }
            w.write(XmlEvent::end_element())?;
            Ok(())
        }
    }
}

pub fn write<W: std::io::Write>(rpc: &XmlRpc, w: &mut EventWriter<W>) -> Result<(), anyhow::Error> {
    use xml::writer::XmlEvent;
    match rpc {
        XmlRpc::MethodCall(method, _) => {
            w.write(XmlEvent::start_element("methodCall"))?;
            w.write(XmlEvent::start_element("methodName"))?;
            w.write(XmlEvent::characters(method))?;
            w.write(XmlEvent::end_element())?;
        }
        XmlRpc::MethodResponse(_) => {
            w.write(XmlEvent::start_element("methodResponse"))?;
        }
    }
    w.write(XmlEvent::start_element("params"))?;
    w.write(XmlEvent::start_element("param"))?;
    w.write(XmlEvent::start_element("value"))?;
    write_inner(rpc.as_ref(), w)?;
    w.write(XmlEvent::end_element())?;
    w.write(XmlEvent::end_element())?;
    w.write(XmlEvent::end_element())?;
    w.write(XmlEvent::end_element())?;
    Ok(())
}

pub fn to_string(rpc: &XmlRpc) -> Result<String, anyhow::Error> {
    let mut buf = Vec::new();
    write(rpc, &mut EventWriter::new(&mut buf))?;
    Ok(String::from_utf8(buf)?)
}

pub fn to_pretty_string(rpc: &XmlRpc) -> Result<String, anyhow::Error> {
    let mut buf = Vec::new();
    write(
        rpc,
        &mut EventWriter::new_with_config(
            &mut buf,
            xml::writer::EmitterConfig::new().perform_indent(true),
        ),
    )?;
    Ok(String::from_utf8(buf)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::collections::HashMap;
    use url::Url;
    use uuid::Uuid;

    fn round_trip(llsd: Llsd) {
        trip(llsd.clone(), llsd);
    }

    fn trip(input: Llsd, output: Llsd) {
        let resp = XmlRpc::new_method_response(input);
        let encoded = to_string(&resp).expect("Failed to encode");
        let decoded = from_str(&encoded).expect("Failed to decode");
        assert_eq!(&output, decoded.llsd());
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
        trip(Llsd::Uri(url.clone().into()), Llsd::String(url.to_string()));
    }

    #[test]
    fn uuid() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        trip(Llsd::Uuid(uuid), Llsd::String(uuid.to_string()));
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
