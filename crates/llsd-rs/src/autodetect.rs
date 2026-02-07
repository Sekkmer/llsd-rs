use std::io::Read;

use crate::{Llsd, binary, notation, xml};

const MAX_HDR_LEN: usize = 20;
const LEGACY_NON_HEADER: &[u8] = b"<llsd>";
const HEADER_BINARY: &str = "LLSD/Binary";
const HEADER_XML: &str = "LLSD/XML";
const HEADER_NOTATION: &str = "llsd/notation";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlsdEncoding {
    Binary,
    Xml,
    Notation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoDecodeOptions {
    pub notation_max_depth: usize,
}

impl Default for AutoDecodeOptions {
    fn default() -> Self {
        Self {
            notation_max_depth: 64,
        }
    }
}

pub fn detect_format(data: &[u8]) -> LlsdEncoding {
    if starts_with_ignore_ascii_case(data, LEGACY_NON_HEADER) {
        return LlsdEncoding::Xml;
    }

    if let Some(token) = detect_header_token(data) {
        if token.eq_ignore_ascii_case(HEADER_BINARY) {
            return LlsdEncoding::Binary;
        }
        if token.eq_ignore_ascii_case(HEADER_XML) {
            return LlsdEncoding::Xml;
        }
        if token.eq_ignore_ascii_case(HEADER_NOTATION) {
            return LlsdEncoding::Notation;
        }
    }

    if data.first() == Some(&b'<') {
        LlsdEncoding::Xml
    } else {
        LlsdEncoding::Notation
    }
}

pub fn from_slice(data: &[u8]) -> Result<Llsd, anyhow::Error> {
    from_slice_with(data, AutoDecodeOptions::default())
}

pub fn from_slice_with(data: &[u8], options: AutoDecodeOptions) -> Result<Llsd, anyhow::Error> {
    let format = detect_format(data);
    let payload = payload_after_header(data, format);
    match format {
        LlsdEncoding::Binary => binary::from_slice(payload),
        LlsdEncoding::Xml => xml::from_slice(payload),
        LlsdEncoding::Notation => notation::from_bytes(payload, options.notation_max_depth)
            .map_err(|err| anyhow::anyhow!("Notation parse error: {err}")),
    }
}

pub fn from_reader<R: Read>(mut reader: R) -> Result<Llsd, anyhow::Error> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    from_slice(&buf)
}

fn starts_with_ignore_ascii_case(data: &[u8], prefix: &[u8]) -> bool {
    data.len() >= prefix.len()
        && data[..prefix.len()]
            .iter()
            .zip(prefix.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

fn detect_header_token(data: &[u8]) -> Option<String> {
    if data.is_empty() {
        return None;
    }
    let header_slice = &data[..data.len().min(MAX_HDR_LEN)];
    let mut header = String::from_utf8_lossy(header_slice).into_owned();
    header = header.trim_end_matches(['\r', '\n']).to_string();
    if header.is_empty() {
        return None;
    }

    let start = header.find(|ch| !matches!(ch, '<' | '?' | ' '))?;
    let rest = &header[start..];
    let end = rest.find([' ', '?']).unwrap_or(rest.len());
    let token = &rest[..end];
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn payload_after_header<'a>(data: &'a [u8], format: LlsdEncoding) -> &'a [u8] {
    if starts_with_ignore_ascii_case(data, LEGACY_NON_HEADER) {
        return data;
    }
    let Some(token) = detect_header_token(data) else {
        return data;
    };
    let recognized = match format {
        LlsdEncoding::Binary => token.eq_ignore_ascii_case(HEADER_BINARY),
        LlsdEncoding::Xml => token.eq_ignore_ascii_case(HEADER_XML),
        LlsdEncoding::Notation => token.eq_ignore_ascii_case(HEADER_NOTATION),
    };
    if !recognized {
        return data;
    }

    let mut offset = if let Some(pos) = find_subslice(data, b"?>") {
        pos + 2
    } else {
        0
    };
    while offset < data.len() && data[offset].is_ascii_whitespace() {
        offset += 1;
    }
    &data[offset..]
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use crate::{Llsd, notation};

    use super::*;

    #[test]
    fn detect_legacy_xml_prefix() {
        assert_eq!(detect_format(b"<llsd><undef/></llsd>"), LlsdEncoding::Xml);
    }

    #[test]
    fn detect_headers_case_insensitive() {
        assert_eq!(detect_format(b"<? llsd/binary ?>\n"), LlsdEncoding::Binary);
        assert_eq!(detect_format(b"<? LLSD/XML ?>\n"), LlsdEncoding::Xml);
        assert_eq!(
            detect_format(b"<? LLSD/NOTATION ?>\n"),
            LlsdEncoding::Notation
        );
    }

    #[test]
    fn detect_fallback_by_first_char() {
        assert_eq!(detect_format(b"<map></map>"), LlsdEncoding::Xml);
        assert_eq!(detect_format(b"{'k':'v'}"), LlsdEncoding::Notation);
    }

    #[test]
    fn parse_binary_with_header() {
        let body = crate::binary::to_vec(&Llsd::Integer(42)).expect("encode binary");
        let mut payload = b"<? LLSD/Binary ?>\n".to_vec();
        payload.extend_from_slice(&body);
        let decoded = from_slice(&payload).expect("decode auto");
        assert_eq!(decoded, Llsd::Integer(42));
    }

    #[test]
    fn parse_xml_with_header() {
        let body = crate::xml::to_string(&Llsd::Integer(7)).expect("encode xml");
        let payload = format!("<? LLSD/XML ?>\n{body}");
        let decoded = from_slice(payload.as_bytes()).expect("decode auto");
        assert_eq!(decoded, Llsd::Integer(7));
    }

    #[test]
    fn parse_notation_with_header() {
        let body = notation::to_vec(&Llsd::Integer(9), &notation::FormatterContext::default())
            .expect("encode notation");
        let mut payload = b"<? llsd/notation ?>\n".to_vec();
        payload.extend_from_slice(&body);
        let decoded = from_slice(&payload).expect("decode auto");
        assert_eq!(decoded, Llsd::Integer(9));
    }
}
