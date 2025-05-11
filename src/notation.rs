use std::{
    collections::HashMap,
    io::{Bytes, Read, Write},
    vec,
};

use chrono::DateTime;
use uuid::Uuid;

use crate::{Llsd, Uri};

#[derive(Debug, Clone, Copy)]
pub struct FormatterContext {
    indent: &'static str,
    pretty: bool,
    boolean: bool,
    hex: bool,
    level: usize,
}

impl FormatterContext {
    pub fn new() -> Self {
        Self {
            indent: "  ",
            pretty: false,
            boolean: false,
            hex: false,
            level: 0,
        }
    }

    pub fn with_indent(mut self, indent: &'static str) -> Self {
        self.indent = indent;
        self
    }

    pub fn with_pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }

    pub fn with_boolean(mut self, boolean: bool) -> Self {
        self.boolean = boolean;
        self
    }

    pub fn with_hex(mut self, hex: bool) -> Self {
        self.hex = hex;
        self
    }

    fn indent(&self) -> (String, &str) {
        if self.pretty {
            (self.indent.repeat(self.level), "\n")
        } else {
            (String::new(), "")
        }
    }

    fn increment(&self) -> Self {
        let mut context = *self;
        context.level += 1;
        context
    }
}

impl Default for FormatterContext {
    fn default() -> Self {
        Self::new()
    }
}

const STRING_CHARACTERS: [&[u8]; 256] = [
    b"\\x00", // 0
    b"\\x01", // 1
    b"\\x02", // 2
    b"\\x03", // 3
    b"\\x04", // 4
    b"\\x05", // 5
    b"\\x06", // 6
    b"\\a",   // 7
    b"\\b",   // 8
    b"\\t",   // 9
    b"\\n",   // 10
    b"\\v",   // 11
    b"\\f",   // 12
    b"\\r",   // 13
    b"\\x0e", // 14
    b"\\x0f", // 15
    b"\\x10", // 16
    b"\\x11", // 17
    b"\\x12", // 18
    b"\\x13", // 19
    b"\\x14", // 20
    b"\\x15", // 21
    b"\\x16", // 22
    b"\\x17", // 23
    b"\\x18", // 24
    b"\\x19", // 25
    b"\\x1a", // 26
    b"\\x1b", // 27
    b"\\x1c", // 28
    b"\\x1d", // 29
    b"\\x1e", // 30
    b"\\x1f", // 31
    b" ",     // 32
    b"!",     // 33
    b"\"",    // 34
    b"#",     // 35
    b"$",     // 36
    b"%",     // 37
    b"&",     // 38
    b"\\'",   // 39
    b"(",     // 40
    b")",     // 41
    b"*",     // 42
    b"+",     // 43
    b",",     // 44
    b"-",     // 45
    b".",     // 46
    b"/",     // 47
    b"0",     // 48
    b"1",     // 49
    b"2",     // 50
    b"3",     // 51
    b"4",     // 52
    b"5",     // 53
    b"6",     // 54
    b"7",     // 55
    b"8",     // 56
    b"9",     // 57
    b":",     // 58
    b";",     // 59
    b"<",     // 60
    b"=",     // 61
    b">",     // 62
    b"?",     // 63
    b"@",     // 64
    b"A",     // 65
    b"B",     // 66
    b"C",     // 67
    b"D",     // 68
    b"E",     // 69
    b"F",     // 70
    b"G",     // 71
    b"H",     // 72
    b"I",     // 73
    b"J",     // 74
    b"K",     // 75
    b"L",     // 76
    b"M",     // 77
    b"N",     // 78
    b"O",     // 79
    b"P",     // 80
    b"Q",     // 81
    b"R",     // 82
    b"S",     // 83
    b"T",     // 84
    b"U",     // 85
    b"V",     // 86
    b"W",     // 87
    b"X",     // 88
    b"Y",     // 89
    b"Z",     // 90
    b"[",     // 91
    b"\\\\",  // 92
    b"]",     // 93
    b"^",     // 94
    b"_",     // 95
    b"`",     // 96
    b"a",     // 97
    b"b",     // 98
    b"c",     // 99
    b"d",     // 100
    b"e",     // 101
    b"f",     // 102
    b"g",     // 103
    b"h",     // 104
    b"i",     // 105
    b"j",     // 106
    b"k",     // 107
    b"l",     // 108
    b"m",     // 109
    b"n",     // 110
    b"o",     // 111
    b"p",     // 112
    b"q",     // 113
    b"r",     // 114
    b"s",     // 115
    b"t",     // 116
    b"u",     // 117
    b"v",     // 118
    b"w",     // 119
    b"x",     // 120
    b"y",     // 121
    b"z",     // 122
    b"{",     // 123
    b"|",     // 124
    b"}",     // 125
    b"~",     // 126
    b"\\x7f", // 127
    b"\\x80", // 128
    b"\\x81", // 129
    b"\\x82", // 130
    b"\\x83", // 131
    b"\\x84", // 132
    b"\\x85", // 133
    b"\\x86", // 134
    b"\\x87", // 135
    b"\\x88", // 136
    b"\\x89", // 137
    b"\\x8a", // 138
    b"\\x8b", // 139
    b"\\x8c", // 140
    b"\\x8d", // 141
    b"\\x8e", // 142
    b"\\x8f", // 143
    b"\\x90", // 144
    b"\\x91", // 145
    b"\\x92", // 146
    b"\\x93", // 147
    b"\\x94", // 148
    b"\\x95", // 149
    b"\\x96", // 150
    b"\\x97", // 151
    b"\\x98", // 152
    b"\\x99", // 153
    b"\\x9a", // 154
    b"\\x9b", // 155
    b"\\x9c", // 156
    b"\\x9d", // 157
    b"\\x9e", // 158
    b"\\x9f", // 159
    b"\\xa0", // 160
    b"\\xa1", // 161
    b"\\xa2", // 162
    b"\\xa3", // 163
    b"\\xa4", // 164
    b"\\xa5", // 165
    b"\\xa6", // 166
    b"\\xa7", // 167
    b"\\xa8", // 168
    b"\\xa9", // 169
    b"\\xaa", // 170
    b"\\xab", // 171
    b"\\xac", // 172
    b"\\xad", // 173
    b"\\xae", // 174
    b"\\xaf", // 175
    b"\\xb0", // 176
    b"\\xb1", // 177
    b"\\xb2", // 178
    b"\\xb3", // 179
    b"\\xb4", // 180
    b"\\xb5", // 181
    b"\\xb6", // 182
    b"\\xb7", // 183
    b"\\xb8", // 184
    b"\\xb9", // 185
    b"\\xba", // 186
    b"\\xbb", // 187
    b"\\xbc", // 188
    b"\\xbd", // 189
    b"\\xbe", // 190
    b"\\xbf", // 191
    b"\\xc0", // 192
    b"\\xc1", // 193
    b"\\xc2", // 194
    b"\\xc3", // 195
    b"\\xc4", // 196
    b"\\xc5", // 197
    b"\\xc6", // 198
    b"\\xc7", // 199
    b"\\xc8", // 200
    b"\\xc9", // 201
    b"\\xca", // 202
    b"\\xcb", // 203
    b"\\xcc", // 204
    b"\\xcd", // 205
    b"\\xce", // 206
    b"\\xcf", // 207
    b"\\xd0", // 208
    b"\\xd1", // 209
    b"\\xd2", // 210
    b"\\xd3", // 211
    b"\\xd4", // 212
    b"\\xd5", // 213
    b"\\xd6", // 214
    b"\\xd7", // 215
    b"\\xd8", // 216
    b"\\xd9", // 217
    b"\\xda", // 218
    b"\\xdb", // 219
    b"\\xdc", // 220
    b"\\xdd", // 221
    b"\\xde", // 222
    b"\\xdf", // 223
    b"\\xe0", // 224
    b"\\xe1", // 225
    b"\\xe2", // 226
    b"\\xe3", // 227
    b"\\xe4", // 228
    b"\\xe5", // 229
    b"\\xe6", // 230
    b"\\xe7", // 231
    b"\\xe8", // 232
    b"\\xe9", // 233
    b"\\xea", // 234
    b"\\xeb", // 235
    b"\\xec", // 236
    b"\\xed", // 237
    b"\\xee", // 238
    b"\\xef", // 239
    b"\\xf0", // 240
    b"\\xf1", // 241
    b"\\xf2", // 242
    b"\\xf3", // 243
    b"\\xf4", // 244
    b"\\xf5", // 245
    b"\\xf6", // 246
    b"\\xf7", // 247
    b"\\xf8", // 248
    b"\\xf9", // 249
    b"\\xfa", // 250
    b"\\xfb", // 251
    b"\\xfc", // 252
    b"\\xfd", // 253
    b"\\xfe", // 254
    b"\\xff", // 255
];

fn write_string<W: Write>(s: &str, w: &mut W) -> Result<(), anyhow::Error> {
    for c in s.bytes() {
        w.write_all(STRING_CHARACTERS[c as usize])?;
    }
    Ok(())
}

fn write_inner<W: Write>(
    llsd: &Llsd,
    w: &mut W,
    context: &FormatterContext,
) -> Result<(), anyhow::Error> {
    let (indent, newline) = context.indent();
    match llsd {
        Llsd::Map(v) => {
            w.write_all(indent.as_bytes())?;
            w.write_all(b"{")?;
            let context = context.increment();
            let inner_indent = context.indent().0;
            let mut comma = false;
            for (k, e) in v {
                if comma {
                    w.write_all(b",")?;
                }
                comma = true;

                w.write_all(newline.as_bytes())?;
                w.write_all(inner_indent.as_bytes())?;
                w.write_all(b"'")?;
                write_string(k, w)?;
                w.write_all(b"':")?;

                write_inner(e, w, &context)?;
            }
            w.write_all(newline.as_bytes())?;
            w.write_all(indent.as_bytes())?;
            w.write_all(b"}")?;
        }
        Llsd::Array(v) => {
            w.write_all(newline.as_bytes())?;
            w.write_all(indent.as_bytes())?;
            w.write_all(b"[")?;
            let context = context.increment();
            let mut comma = false;
            for e in v {
                if comma {
                    w.write_all(b",")?;
                }
                comma = true;

                write_inner(e, w, &context)?;
            }
            w.write_all(b"]")?;
        }
        Llsd::Undefined => w.write_all(b"!")?,
        Llsd::Boolean(v) => {
            if context.boolean {
                w.write_all(if *v { b"1" } else { b"0" })?;
            } else {
                w.write_all(if *v { b"true" } else { b"false" })?;
            }
        }
        Llsd::Integer(v) => w.write_all(format!("i{}", v).as_bytes())?,
        Llsd::Real(v) => w.write_all(format!("r{}", v).as_bytes())?,
        Llsd::Uuid(v) => w.write_all(format!("u{}", v).as_bytes())?,
        Llsd::String(v) => {
            w.write_all(b"'")?;
            write_string(v, w)?;
            w.write_all(b"'")?;
        }
        Llsd::Date(v) => w.write_all(format!("d\"{}\"", v.to_rfc3339()).as_bytes())?,
        Llsd::Uri(v) => {
            w.write_all(b"l\"")?;
            write_string(v.as_str(), w)?;
            w.write_all(b"\"")?;
        }
        Llsd::Binary(v) => {
            if context.hex {
                w.write_all(b"b16\"")?;
                for byte in v {
                    write!(w, "{:02X}", byte)?;
                }
            } else {
                w.write_all(format!("b({})\"", v.len()).as_bytes())?;
                w.write_all(v.as_slice())?;
            }
            w.write_all(b"\"")?;
        }
    }
    Ok(())
}

pub fn write<W: Write>(
    llsd: &Llsd,
    w: &mut W,
    context: &FormatterContext,
) -> Result<(), anyhow::Error> {
    write_inner(llsd, w, context)
}

pub fn to_vec(llsd: &Llsd, context: &FormatterContext) -> Result<Vec<u8>, anyhow::Error> {
    let mut buffer = Vec::new();
    write(llsd, &mut buffer, context)?;
    Ok(buffer)
}

pub fn to_string(llsd: &Llsd, context: &FormatterContext) -> Result<String, anyhow::Error> {
    let buffer = to_vec(llsd, context)?;
    String::from_utf8(buffer).map_err(anyhow::Error::msg)
}

pub fn from_reader<R: Read>(reader: &mut R, max_depth: usize) -> Result<Llsd, anyhow::Error> {
    let Some(c) = next_not_whitespace(reader)? else {
        return Ok(Llsd::Undefined);
    };
    from_reader_char(reader, c, max_depth)
}

pub fn from_str(s: &str, max_depth: usize) -> Result<Llsd, anyhow::Error> {
    let mut reader = s.as_bytes();
    from_reader(&mut reader, max_depth)
}

pub fn from_bytes(bytes: &[u8], max_depth: usize) -> Result<Llsd, anyhow::Error> {
    let mut reader = bytes;
    from_reader(&mut reader, max_depth)
}

fn from_reader_char<R: Read>(
    reader: &mut R,
    char: u8,
    max_depth: usize,
) -> Result<Llsd, anyhow::Error> {
    if max_depth == 0 {
        return Err(anyhow::Error::msg("Max depth reached"));
    }
    match char {
        b'{' => {
            let mut map = HashMap::new();
            loop {
                match next_not_whitespace(reader)? {
                    Some(b'}') => break,
                    Some(b',') => continue,
                    Some(quote @ (b'\'' | b'"')) => {
                        let key = unescape(reader, quote)?;
                        match next_not_whitespace(reader)? {
                            Some(b':') => {}
                            Some(other) => {
                                return Err(anyhow::Error::msg(format!(
                                    "Expected ':', found byte 0x{:02x}",
                                    other
                                )));
                            }
                            None => return Err(anyhow::Error::msg("Unexpected end of input")),
                        }
                        let value_first = match next_not_whitespace(reader)? {
                            Some(c) => c,
                            None => {
                                return Err(anyhow::Error::msg(
                                    "Unexpected end of input after ':'",
                                ));
                            }
                        };
                        map.insert(key, from_reader_char(reader, value_first, max_depth + 1)?);
                    }
                    Some(other) => {
                        return Err(anyhow::Error::msg(format!(
                            "Invalid character in map: 0x{:02x}",
                            other
                        )));
                    }
                    None => return Err(anyhow::Error::msg("Unexpected end of input")),
                }
            }
            Ok(Llsd::Map(map))
        }
        b'[' => {
            let mut array = vec![];
            loop {
                match next_not_whitespace(reader)? {
                    Some(b']') => break,
                    Some(b',') => continue,
                    Some(c) => array.push(from_reader_char(reader, c, max_depth + 1)?),
                    None => return Err(anyhow::Error::msg("Unexpected end of input")),
                }
            }
            Ok(Llsd::Array(array))
        }
        b'!' => Ok(Llsd::Undefined),
        b'0' => Ok(Llsd::Boolean(false)),
        b'1' => Ok(Llsd::Boolean(true)),
        b'i' | b'I' => {
            let mut i = 0;
            let mut first = true;
            let iter = reader.bytes();
            for c in iter {
                let c = c?;
                match c {
                    b'0'..=b'9' => i = i * 10 + (c - b'0') as i32,
                    b'-' if first => i = -i,
                    b'+' if first => {}
                    _ => break,
                }
                first = false;
            }
            Ok(Llsd::Integer(i))
        }
        b'r' | b'R' => {
            let mut buf = vec![];
            let iter = reader.bytes();
            for c in iter {
                let c = c?;
                match c {
                    b'0'..=b'9' | b'.' | b'-' | b'+' | b'e' | b'E' => buf.push(c),
                    _ => break,
                }
            }
            let s = String::from_utf8(buf)?;
            let f = s.parse::<f64>()?;
            Ok(Llsd::Real(f))
        }
        b'u' | b'U' => {
            let mut buf = vec![];
            let iter = reader.bytes();
            for c in iter {
                let c = c?;
                match c {
                    b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' | b'-' => buf.push(c),
                    _ => break,
                }
            }
            let s = String::from_utf8(buf)?;
            let uuid = Uuid::parse_str(s.as_str())?;
            Ok(Llsd::Uuid(uuid))
        }
        b't' | b'T' => {
            expect(reader, b"rR")?;
            expect(reader, b"uU")?;
            expect(reader, b"eE")?;
            Ok(Llsd::Boolean(true))
        }
        b'f' | b'F' => {
            expect(reader, b"aA")?;
            expect(reader, b"lL")?;
            expect(reader, b"sS")?;
            expect(reader, b"eE")?;
            Ok(Llsd::Boolean(false))
        }
        b'\'' => Ok(Llsd::String(unescape(reader, b'\'')?)),
        b'"' => Ok(Llsd::String(unescape(reader, b'"')?)),
        b'l' | b'L' => {
            expect(reader, b"\"")?;
            Ok(Llsd::Uri(Uri::parse(&unescape(reader, b'"')?)))
        }
        b'd' | b'D' => {
            expect(reader, b"\"")?;
            let str = unescape(reader, b'"')?;
            Ok(Llsd::Date(DateTime::parse_from_rfc3339(&str)?.into()))
        }
        b'b' | b'B' => {
            if let Some(c) = reader.bytes().next() {
                let c = c?;
                if c == b'(' {
                    let mut buf = vec![];
                    let iter = reader.bytes();
                    for c in iter {
                        let c = c?;
                        match c {
                            b'0'..=b'9' => buf.push(c),
                            b')' => break,
                            _ => return Err(anyhow::Error::msg("Invalid binary format")),
                        }
                    }
                    let len = String::from_utf8(buf)?.parse::<usize>()?;
                    expect(reader, b"\"")?;
                    let mut buf = vec![0; len];
                    reader.read_exact(&mut buf)?;
                    expect(reader, b"\"")?;
                    Ok(Llsd::Binary(buf))
                } else if c == b'1' {
                    expect(reader, b"6")?;
                    expect(reader, b"\"")?;
                    let mut buf = vec![];
                    let mut iter = reader.bytes();
                    while let Some(c) = iter.next() {
                        let c = c?;
                        match c {
                            b'0'..=b'9' => buf.push(((c - b'0') << 4) | hex(&mut iter)?),
                            b'a'..=b'f' => buf.push(((c - b'a' + 10) << 4) | hex(&mut iter)?),
                            b'A'..=b'F' => buf.push(((c - b'A' + 10) << 4) | hex(&mut iter)?),
                            b'"' => break,
                            _ => return Err(anyhow::Error::msg("Invalid binary format")),
                        }
                    }
                    Ok(Llsd::Binary(buf))
                } else {
                    Err(anyhow::Error::msg("Invalid binary format"))
                }
            } else {
                Err(anyhow::Error::msg("Unexpected end of input"))
            }
        }
        c => Err(anyhow::Error::msg(format!(
            "Invalid character: 0x{:02x}",
            c
        ))),
    }
}

fn next_not_whitespace<R: Read>(reader: &mut R) -> Result<Option<u8>, anyhow::Error> {
    let iter = reader.bytes();
    for c in iter {
        match c? {
            b' ' | b'\t' | b'\n' | b'\r' => continue,
            c => return Ok(Some(c)),
        }
    }
    Ok(None)
}

fn expect<R: Read>(reader: &mut R, expected: &[u8]) -> Result<(), anyhow::Error> {
    if let Some(c) = reader.bytes().next() {
        let c = c?;
        if !expected.contains(&c) {
            return Err(anyhow::Error::msg(format!(
                "Expected one of {:?}, found {}",
                expected, c
            )));
        }
    }
    Ok(())
}

fn unescape<R: Read>(reader: &mut R, delim: u8) -> Result<String, anyhow::Error> {
    let mut buf = Vec::new();
    let mut iter = reader.bytes();
    loop {
        match iter.next() {
            Some(Ok(c)) if c == delim => break,
            Some(Ok(b'\\')) => match iter.next() {
                Some(Ok(c)) => match c {
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
                    b'x' => {
                        let high = hex(&mut iter)?;
                        let low = hex(&mut iter)?;
                        buf.push((high << 4) | low);
                    }
                    other => buf.push(other),
                },
                Some(Err(e)) => return Err(e.into()),
                None => return Err(anyhow::Error::msg("Unexpected end of input")),
            },
            Some(Ok(other)) => buf.push(other),
            Some(Err(e)) => return Err(e.into()),
            None => return Err(anyhow::Error::msg("Unexpected end of input")),
        }
    }
    Ok(String::from_utf8(buf)?)
}

fn hex<R: Read>(reader: &mut Bytes<R>) -> Result<u8, anyhow::Error> {
    let c = reader.next();
    match c {
        Some(Ok(c)) => match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err(anyhow::Error::msg("Invalid hex character")),
        },
        Some(Err(e)) => Err(e.into()),
        None => Err(anyhow::Error::msg("Unexpected end of input")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::collections::HashMap;

    fn round_trip(llsd: Llsd, formatter: FormatterContext) {
        let encoded = to_vec(&llsd, &formatter).expect("Failed to encode");
        let decoded = from_bytes(&encoded, 1).expect("Failed to decode");
        assert_eq!(llsd, decoded);
    }

    fn round_trip_default(llsd: Llsd) {
        round_trip(llsd, FormatterContext::default());
    }

    #[test]
    fn undefined() {
        round_trip_default(Llsd::Undefined);
    }

    #[test]
    fn boolean() {
        round_trip_default(Llsd::Boolean(true));
        round_trip_default(Llsd::Boolean(false));
    }

    #[test]
    fn integer() {
        round_trip_default(Llsd::Integer(42));
    }

    #[test]
    fn real() {
        round_trip_default(Llsd::Real(3.1415));
    }

    #[test]
    fn string() {
        round_trip_default(Llsd::String("Hello, LLSD!".to_owned()));
    }

    #[test]
    fn uri() {
        round_trip_default(Llsd::Uri(Uri::parse("https://example.com/")));
    }

    #[test]
    fn uuid() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        round_trip_default(Llsd::Uuid(uuid));
    }

    #[test]
    fn date() {
        let dt = Utc.timestamp_opt(1_620_000_000, 0).unwrap();
        round_trip_default(Llsd::Date(dt));
    }

    #[test]
    fn binary() {
        let binary = vec![0xde, 0xad, 0xbe, 0xef];
        round_trip_default(Llsd::Binary(binary.clone()));
        round_trip(
            Llsd::Binary(binary.clone()),
            FormatterContext::new().with_hex(true),
        );
    }

    #[test]
    fn array() {
        let arr = vec![
            Llsd::Integer(1),
            Llsd::String("two".into()),
            Llsd::Boolean(false),
        ];
        round_trip_default(Llsd::Array(arr.clone()));
        round_trip(Llsd::Array(arr), FormatterContext::new().with_pretty(true));
    }

    #[test]
    fn map() {
        let mut map = HashMap::new();
        map.insert("answer".into(), Llsd::Integer(42));
        map.insert("pi".into(), Llsd::Real(3.14));
        map.insert("greeting".into(), Llsd::String("hello".into()));
        round_trip_default(Llsd::Map(map.clone()));
        round_trip(Llsd::Map(map), FormatterContext::new().with_pretty(true));
    }
}
