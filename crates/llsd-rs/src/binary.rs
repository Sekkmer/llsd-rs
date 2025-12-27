use std::io::{BufRead, BufReader, Read, Write};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{Llsd, Uri};

fn write_inner<W: Write>(llsd: &Llsd, w: &mut W) -> Result<(), anyhow::Error> {
    match llsd {
        Llsd::Undefined => w.write_all(b"!")?,
        Llsd::Boolean(v) => w.write_all(if *v { b"1" } else { b"0" })?,
        Llsd::Integer(v) => {
            w.write_all(b"i")?;
            w.write_all(&v.to_be_bytes())?;
        }
        Llsd::Real(v) => {
            w.write_all(b"r")?;
            w.write_all(&v.to_be_bytes())?;
        }
        Llsd::String(v) => {
            w.write_all(b"s")?;
            w.write_all(&(v.len() as u32).to_be_bytes())?;
            w.write_all(v.as_bytes())?;
        }
        Llsd::Uri(v) => {
            w.write_all(b"l")?;
            let v = v.as_str();
            w.write_all(&(v.len() as u32).to_be_bytes())?;
            w.write_all(v.as_bytes())?;
        }
        Llsd::Uuid(v) => {
            w.write_all(b"u")?;
            w.write_all((*v).as_bytes())?;
        }
        Llsd::Date(v) => {
            w.write_all(b"d")?;
            let real: f64 =
                v.timestamp() as f64 + (v.timestamp_subsec_nanos() as f64 / 1_000_000_000.0);
            // Use little endian
            w.write_all(&real.to_le_bytes())?;
        }
        Llsd::Binary(v) => {
            w.write_all(b"b")?;
            w.write_all(&(v.len() as u32).to_be_bytes())?;
            w.write_all(v)?;
        }
        Llsd::Array(v) => {
            w.write_all(b"[")?;
            w.write_all(&(v.len() as u32).to_be_bytes())?;
            for e in v {
                write_inner(e, w)?;
            }
            w.write_all(b"]")?;
        }
        Llsd::Map(v) => {
            w.write_all(b"{")?;
            w.write_all(&(v.len() as u32).to_be_bytes())?;
            for (k, e) in v {
                w.write_all(b"k")?;
                w.write_all(&(k.len() as u32).to_be_bytes())?;
                w.write_all(k.as_bytes())?;
                write_inner(e, w)?;
            }
            w.write_all(b"}")?;
        }
    }
    Ok(())
}

pub fn write<W: Write>(llsd: &Llsd, w: &mut W) -> Result<(), anyhow::Error> {
    write_inner(llsd, w)
}

pub fn to_vec(llsd: &Llsd) -> Result<Vec<u8>, anyhow::Error> {
    let mut buf = Vec::new();
    write(llsd, &mut buf)?;
    Ok(buf)
}

macro_rules! read_be_fn {
    ($func_name:ident, $type:ty) => {
        fn $func_name<R: Read>(reader: &mut R) -> Result<$type, anyhow::Error> {
            let mut buf = [0_u8; std::mem::size_of::<$type>()];
            reader.read_exact(&mut buf)?;
            Ok(<$type>::from_be_bytes(buf))
        }
    };
}

read_be_fn!(read_u8, u8);
read_be_fn!(read_i32_be, i32);
read_be_fn!(read_f64_be, f64);

fn hex<R: Read>(r: &mut R) -> Result<u8, anyhow::Error> {
    let c = read_u8(r)?;
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Ok(0),
    }
}

fn unescape<R: Read>(r: &mut R, delim: u8) -> Result<String, anyhow::Error> {
    let mut buf = Vec::new();
    loop {
        match read_u8(r)? {
            c if c == delim => break,
            b'\\' => match read_u8(r)? {
                b'a' => buf.push(0x07),
                b'b' => buf.push(0x08),
                b'f' => buf.push(0x0c),
                b'n' => buf.push(b'\n'),
                b'r' => buf.push(b'\r'),
                b't' => buf.push(b'\t'),
                b'v' => buf.push(0x0b),
                b'\\' => buf.push(b'\\'),
                b'\'' => buf.push(b'\''),
                b'"' => buf.push(b'"'),
                b'x' => buf.push((hex(r)? << 4) | hex(r)?),
                other => buf.push(other),
            },
            other => buf.push(other),
        }
    }
    Ok(String::from_utf8(buf)?)
}

pub fn from_reader_inner<R: Read>(r: &mut R) -> Result<Llsd, anyhow::Error> {
    match read_u8(r)? {
        b'!' => Ok(Llsd::Undefined),
        b'1' => Ok(Llsd::Boolean(true)),
        b'0' => Ok(Llsd::Boolean(false)),
        b'i' => Ok(Llsd::Integer(read_i32_be(r)?)),
        b'r' => Ok(Llsd::Real(read_f64_be(r)?)),
        b's' => {
            let len = read_i32_be(r)? as usize;
            let mut buf = vec![0; len];
            r.read_exact(&mut buf)?;
            Ok(Llsd::String(String::from_utf8(buf)?))
        }
        b'l' => {
            let len = read_i32_be(r)? as usize;
            let mut buf = vec![0; len];
            r.read_exact(&mut buf)?;
            Ok(Llsd::Uri(Uri::parse(std::str::from_utf8(&buf)?)))
        }
        b'u' => {
            let mut buf = [0_u8; 16];
            r.read_exact(&mut buf)?;
            Ok(Llsd::Uuid(Uuid::from_slice(&buf)?))
        }
        b'd' => {
            let mut buf = [0_u8; 8];
            r.read_exact(&mut buf)?;
            // Use little endian
            let real = f64::from_le_bytes(buf);
            let date = DateTime::<Utc>::from_timestamp(
                real.trunc() as i64,
                (real.fract() * 1_000_000_000.0) as u32,
            );
            Ok(Llsd::Date(date.unwrap_or_default()))
        }
        b'b' => {
            let len = read_i32_be(r)? as usize;
            let mut buf = vec![0; len];
            r.read_exact(&mut buf)?;
            Ok(Llsd::Binary(buf))
        }
        b'[' => {
            let len = read_i32_be(r)? as usize;
            let mut buf = Vec::with_capacity(len);
            for _ in 0..len {
                buf.push(from_reader_inner(r)?);
            }
            if read_u8(r)? != b']' {
                return Err(anyhow::anyhow!("Expected ']'"));
            }
            Ok(Llsd::Array(buf))
        }
        b'{' => {
            let len = read_i32_be(r)? as usize;
            let mut buf = std::collections::HashMap::with_capacity(len);
            for _ in 0..len {
                if read_u8(r)? != b'k' {
                    return Err(anyhow::anyhow!("Expected 'k'"));
                }
                let key_len = read_i32_be(r)? as usize;
                let mut key_buf = vec![0; key_len];
                r.read_exact(&mut key_buf)?;
                let key = String::from_utf8(key_buf)?;
                let value = from_reader_inner(r)?;
                buf.insert(key, value);
            }
            if read_u8(r)? != b'}' {
                return Err(anyhow::anyhow!("Expected '}}'"));
            }
            Ok(Llsd::Map(buf))
        }
        b'"' => Ok(Llsd::String(unescape(r, b'"')?)),
        b'\'' => Ok(Llsd::String(unescape(r, b'\'')?)),
        other => Err(anyhow::anyhow!("Unknown LLSD type: {}", other)),
    }
}

fn looks_like_llsd_binary_header(header: &[u8]) -> bool {
    header
        .windows(b"LLSD/Binary".len())
        .any(|w| w == b"LLSD/Binary")
}

pub fn from_reader<R: Read>(r: &mut R) -> Result<Llsd, anyhow::Error> {
    let mut reader = BufReader::new(r);
    {
        let buf = reader.fill_buf()?;
        if matches!(buf.first(), Some(b'<')) {
            let mut header = Vec::new();
            reader.read_until(b'>', &mut header)?;
            if looks_like_llsd_binary_header(&header) {
                loop {
                    let next = reader.fill_buf()?;
                    match next.first() {
                        Some(b' ' | b'\r' | b'\n' | b'\t') => reader.consume(1),
                        _ => break,
                    }
                }
            } else {
                return Err(anyhow::anyhow!("Unexpected LLSD binary header"));
            }
        }
    }
    from_reader_inner(&mut reader)
}

pub fn from_slice(data: &[u8]) -> Result<Llsd, anyhow::Error> {
    from_reader(&mut std::io::Cursor::new(data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::collections::HashMap;

    fn round_trip(llsd: Llsd) {
        let encoded = to_vec(&llsd).expect("Failed to encode");
        let decoded = from_slice(&encoded).expect("Failed to decode");
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
        round_trip(Llsd::Uri(Uri::parse("https://example.com/")));
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
    fn array_in_map_parses_closing_bracket() {
        let mut map = HashMap::new();
        map.insert(
            "a".to_string(),
            Llsd::Array(vec![Llsd::Integer(1), Llsd::Integer(2)]),
        );
        map.insert("b".to_string(), Llsd::String("ok".to_string()));

        let encoded = to_vec(&Llsd::Map(map.clone())).expect("encode failed");
        let decoded = from_slice(&encoded).expect("decode failed");
        assert_eq!(decoded, Llsd::Map(map));
    }

    #[test]
    fn binary_header_prefix_is_skipped() {
        let value = Llsd::String("hello".to_string());
        let mut encoded = b"<? LLSD/Binary ?>\n".to_vec();
        encoded.extend(to_vec(&value).expect("encode failed"));

        let decoded = from_slice(&encoded).expect("decode failed");
        assert_eq!(decoded, value);
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
