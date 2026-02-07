#![cfg(feature = "derive")]
use llsd_rs::{Llsd, LlsdFrom, LlsdFromTo};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, LlsdFromTo)]
struct Simple {
    id: u32,
    #[llsd(default)]
    name: Option<String>,
}

#[test]
fn simple_round_trip() {
    let s = Simple {
        id: 7,
        name: Some("Alice".into()),
    };
    let l: Llsd = s.clone().into();
    let back: Simple = Simple::try_from(&l).unwrap();
    assert_eq!(s, back);
}

#[test]
fn option_default_missing_field_stays_none() {
    let l = Llsd::map().insert("id", 7u32).unwrap();
    let parsed: Simple = Simple::try_from(&l).unwrap();
    assert_eq!(parsed.id, 7);
    assert_eq!(parsed.name, None);
}

#[derive(Debug, Clone, PartialEq, LlsdFromTo)]
#[llsd(rename_all = "camelCase", deny_unknown_fields)]
struct Collections {
    numbers: Vec<i32>,
    #[llsd(rename = "dataMap")]
    data: HashMap<String, Inner>,
    tuple: (i32, String),
}

#[derive(Debug, Clone, PartialEq, LlsdFromTo)]
struct Inner {
    value: i32,
}

#[test]
fn collections_round_trip() {
    let mut map = HashMap::new();
    map.insert("first".to_string(), Inner { value: 10 });
    map.insert("second".to_string(), Inner { value: 20 });
    let c = Collections {
        numbers: vec![1, 2, 3],
        data: map,
        tuple: (5, "hi".into()),
    };
    let l: Llsd = c.clone().into();
    let back: Collections = Collections::try_from(&l).unwrap();
    assert_eq!(c, back);
}

#[derive(Debug, Clone, PartialEq, LlsdFrom)]
#[llsd(rename_all = "snake_case", deny_unknown_fields)]
struct RenameAndDefault {
    #[llsd(rename = "UserID")]
    user_id: u32,
    #[llsd(default = default_name)]
    name: String,
}

fn default_name() -> String {
    "Bob".into()
}

#[test]
fn rename_and_default_missing_field() {
    let l = Llsd::map().insert("UserID", 100u32).unwrap();
    let r: RenameAndDefault = RenameAndDefault::try_from(&l).unwrap();
    assert_eq!(r.user_id, 100);
    assert_eq!(r.name, "Bob");
}

#[derive(Debug, Clone, PartialEq, LlsdFromTo)]
struct FlattenOuter {
    id: u32,
    #[llsd(flatten)]
    inner: FlattenInner,
}
#[derive(Debug, Clone, PartialEq, LlsdFromTo)]
struct FlattenInner {
    a: i32,
    b: i32,
}

#[test]
fn flatten_merge() {
    let o = FlattenOuter {
        id: 1,
        inner: FlattenInner { a: 2, b: 3 },
    };
    let l: Llsd = o.clone().into();
    let map = l.as_map().unwrap();
    assert!(map.contains_key("id"));
    assert!(map.contains_key("a"));
    assert!(map.contains_key("b"));
    let back: FlattenOuter = FlattenOuter::try_from(&l).unwrap();
    assert_eq!(o, back);
}

#[test]
fn tuple_try_from() {
    let l = Llsd::Array(vec![1i32.into(), "hi".into()]);
    let t: (i32, String) = <(i32, String)>::try_from(&l).unwrap();
    assert_eq!(t.0, 1);
    assert_eq!(t.1, "hi");
}

mod custom_u32_as_string {
    use llsd_rs::Llsd;
    pub fn serialize(v: &u32) -> Llsd {
        Llsd::from(v.to_string())
    }
    pub fn deserialize(v: &Llsd) -> anyhow::Result<u32> {
        match v {
            Llsd::String(s) => s.parse::<u32>().map_err(|_| anyhow::Error::msg("bad int")),
            _ => Err(anyhow::Error::msg("expected string")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, LlsdFromTo)]
struct WithDemo {
    id: u32,
    #[llsd(with = custom_u32_as_string)]
    code: u32,
}

#[test]
fn with_attribute_round_trip() {
    let w = WithDemo { id: 9, code: 42 };
    let l: Llsd = w.clone().into();
    let map = l.as_map().unwrap();
    assert_eq!(map.get("code").unwrap().as_string().unwrap(), "42");
    let back: WithDemo = WithDemo::try_from(&l).unwrap();
    assert_eq!(w, back);
}
