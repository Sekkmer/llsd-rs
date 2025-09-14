use std::{collections::HashMap, ops};

use anyhow::Result;
use chrono::{DateTime, FixedOffset, Utc};
use enum_as_inner::EnumAsInner;
use url::Url;
use uuid::Uuid;

pub mod binary;
pub mod derive;
pub mod notation;
pub mod rpc;
pub mod xml;

#[cfg(feature = "derive")]
pub use llsd_rs_derive::{LlsdFrom, LlsdFromTo, LlsdInto};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Uri {
    #[default]
    Empty,
    Url(Url),
    String(String, url::ParseError),
}

impl Uri {
    pub fn new() -> Self {
        Uri::Empty
    }

    pub fn parse(uri: &str) -> Self {
        let uri = uri.trim();
        if uri.is_empty() {
            return Uri::Empty;
        }
        match Url::parse(uri) {
            Ok(url) => Uri::Url(url),
            Err(e) => Uri::String(uri.to_string(), e),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Uri::Url(url) => url.as_str(),
            Uri::String(s, _) => s,
            Uri::Empty => "",
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Uri::Empty)
    }

    pub fn is_url(&self) -> bool {
        matches!(self, Uri::Url(_))
    }

    pub fn error(&self) -> Option<url::ParseError> {
        match self {
            Uri::String(_, e) => Some(*e),
            _ => None,
        }
    }
}

impl From<Url> for Uri {
    fn from(uri: Url) -> Self {
        Uri::Url(uri)
    }
}

impl From<&str> for Uri {
    fn from(uri: &str) -> Self {
        Self::parse(uri)
    }
}

impl From<String> for Uri {
    fn from(uri: String) -> Self {
        Self::parse(&uri)
    }
}

impl From<&Uri> for String {
    fn from(uri: &Uri) -> Self {
        match uri {
            Uri::Url(url) => url.to_string(),
            Uri::String(s, _) => s.clone(),
            Uri::Empty => String::new(),
        }
    }
}

impl<'a> From<&'a Uri> for &'a str {
    fn from(uri: &'a Uri) -> Self {
        match uri {
            Uri::Url(url) => url.as_str(),
            Uri::String(s, _) => s,
            Uri::Empty => "",
        }
    }
}

impl TryFrom<&Uri> for Url {
    type Error = url::ParseError;

    fn try_from(uri: &Uri) -> core::result::Result<Self, Self::Error> {
        match uri {
            Uri::Url(url) => Ok(url.clone()),
            Uri::String(_, e) => Err(*e),
            Uri::Empty => Err(url::ParseError::EmptyHost),
        }
    }
}

#[derive(Debug, Default, Clone, EnumAsInner, PartialEq)]
pub enum Llsd {
    #[default]
    Undefined,
    Boolean(bool),
    Integer(i32),
    Real(f64),
    String(String),
    Uri(Uri),
    Uuid(Uuid),
    Date(DateTime<Utc>),
    Binary(Vec<u8>),
    Array(Vec<Llsd>),
    Map(HashMap<String, Llsd>),
}

impl Llsd {
    pub fn new() -> Self {
        Llsd::Undefined
    }

    pub fn array() -> Self {
        Llsd::Array(Vec::new())
    }

    pub fn map() -> Self {
        Llsd::Map(HashMap::new())
    }

    pub fn clear(&mut self) {
        *self = Llsd::Undefined;
    }

    pub fn push<T: Into<Llsd>>(mut self, llsd: T) -> Result<Self> {
        match &mut self {
            Llsd::Array(array) => array.push(llsd.into()),
            Llsd::Undefined => {
                self = Llsd::Array(vec![llsd.into()]);
            }
            _ => return Err(anyhow::Error::msg("not an array")),
        }
        Ok(self)
    }

    pub fn insert<K: Into<String>, T: Into<Llsd>>(mut self, key: K, llsd: T) -> Result<Self> {
        match &mut self {
            Llsd::Map(map) => {
                map.insert(key.into(), llsd.into());
            }
            Llsd::Undefined => {
                let mut map = HashMap::new();
                map.insert(key.into(), llsd.into());
                self = Llsd::Map(map);
            }
            _ => return Err(anyhow::Error::msg("not a map")),
        }
        Ok(self)
    }

    pub fn get(&self, index: impl Index) -> Option<&Llsd> {
        index.index_into(self)
    }

    pub fn get_mut(&mut self, index: impl Index) -> Option<&mut Llsd> {
        index.index_into_mut(self)
    }

    pub fn contains(&self, index: impl Index) -> bool {
        self.get(index).is_some()
    }

    pub fn len(&self) -> usize {
        match self {
            Llsd::Array(a) => a.len(),
            Llsd::Map(m) => m.len(),
            _ => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn pointer(&self, pointer: &str) -> Option<&Llsd> {
        if pointer.is_empty() {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        pointer
            .split('/')
            .skip(1)
            .map(|x| x.replace("~1", "/").replace("~0", "~"))
            .try_fold(self, |target, token| match target {
                Llsd::Array(array) => token.parse::<usize>().ok().and_then(|x| array.get(x)),
                Llsd::Map(map) => map.get(&token),
                _ => None,
            })
    }

    pub fn pointer_mut(&mut self, pointer: &str) -> Option<&mut Llsd> {
        if pointer.is_empty() {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        pointer
            .split('/')
            .skip(1)
            .map(|x| x.replace("~1", "/").replace("~0", "~"))
            .try_fold(self, |target, token| match target {
                Llsd::Array(array) => token.parse::<usize>().ok().and_then(|x| array.get_mut(x)),
                Llsd::Map(map) => map.get_mut(&token),
                _ => None,
            })
    }

    pub fn take(&mut self) -> Self {
        std::mem::replace(self, Llsd::Undefined)
    }
}

impl From<bool> for Llsd {
    fn from(llsd: bool) -> Self {
        Llsd::Boolean(llsd)
    }
}

impl From<&bool> for Llsd {
    fn from(v: &bool) -> Self {
        Llsd::Boolean(*v)
    }
}

macro_rules! impl_from_int {
    ($($t:ty),*) => {
            $(
            impl From<$t> for Llsd {
                fn from(llsd: $t) -> Self {
                    Llsd::Integer(llsd as i32)
                }
            }
            impl TryFrom<&Llsd> for $t {
                type Error = anyhow::Error;

                fn try_from(llsd: &Llsd) -> Result<Self> {
                    match llsd {
                        Llsd::Integer(value) => Ok(*value as $t),
                        Llsd::Real(value) => Ok(*value as $t),
                        Llsd::Boolean(value) => Ok(if *value { 1 } else { 0 } as $t),
                        Llsd::String(value) => {
                            value.parse::<$t>().map_err(|_| anyhow::Error::msg("Invalid integer"))
                        }
                        _ => Err(anyhow::Error::msg("Expected LLSD Integer")),
                    }
                }
            }
		)*
	};
}

impl_from_int!(u8, u16, u32, u64, i8, i16, i32, i64);

macro_rules! impl_from_real {
    ($($t:ty),*) => {
        $(
            impl From<$t> for Llsd {
                fn from(llsd: $t) -> Self {
                    Llsd::Real(llsd as f64)
                }
            }
            impl From<&$t> for Llsd {
                fn from(llsd: &$t) -> Self {
                    Llsd::Real(*llsd as f64)
                }
            }
            impl TryFrom<&Llsd> for $t {
                type Error = anyhow::Error;

                fn try_from(llsd: &Llsd) -> Result<Self> {
                    match llsd {
                        Llsd::Real(value) => Ok(*value as $t),
                        Llsd::Integer(value) => Ok(*value as $t),
                        Llsd::Boolean(value) => Ok(if *value { 1.0 } else { 0.0 } as $t),
                        Llsd::String(value) => {
                            value.parse::<$t>().map_err(|_| anyhow::Error::msg("Invalid real"))
                        }
                        _ => Err(anyhow::Error::msg("Expected LLSD Real")),
                    }
                }
            }
        )*
    };
}

impl_from_real!(f32, f64);

impl From<&str> for Llsd {
    fn from(llsd: &str) -> Self {
        Llsd::String(llsd.to_string())
    }
}

impl From<String> for Llsd {
    fn from(llsd: String) -> Self {
        Llsd::String(llsd)
    }
}

impl From<&String> for Llsd {
    fn from(v: &String) -> Self {
        Llsd::String(v.clone())
    }
}

impl From<Uuid> for Llsd {
    fn from(llsd: Uuid) -> Self {
        Llsd::Uuid(llsd)
    }
}

impl From<&Uuid> for Llsd {
    fn from(v: &Uuid) -> Self {
        Llsd::Uuid(*v)
    }
}

impl From<Url> for Llsd {
    fn from(llsd: Url) -> Self {
        Llsd::Uri(llsd.into())
    }
}

impl From<&Url> for Llsd {
    fn from(v: &Url) -> Self {
        Llsd::Uri(v.clone().into())
    }
}

impl From<DateTime<Utc>> for Llsd {
    fn from(llsd: DateTime<Utc>) -> Self {
        Llsd::Date(llsd)
    }
}

impl From<&DateTime<Utc>> for Llsd {
    fn from(v: &DateTime<Utc>) -> Self {
        Llsd::Date(*v)
    }
}

impl From<DateTime<FixedOffset>> for Llsd {
    fn from(llsd: DateTime<FixedOffset>) -> Self {
        Llsd::Date(llsd.with_timezone(&Utc))
    }
}

impl From<&DateTime<FixedOffset>> for Llsd {
    fn from(v: &DateTime<FixedOffset>) -> Self {
        Llsd::Date(v.with_timezone(&Utc))
    }
}

impl From<&[u8]> for Llsd {
    fn from(llsd: &[u8]) -> Self {
        Llsd::Binary(Vec::from(llsd))
    }
}

impl<const N: usize> From<[u8; N]> for Llsd {
    fn from(llsd: [u8; N]) -> Self {
        Llsd::Binary(llsd.to_vec())
    }
}

impl<T: Into<Llsd>> From<Vec<T>> for Llsd {
    fn from(llsd: Vec<T>) -> Self {
        Llsd::Array(llsd.into_iter().map(Into::into).collect())
    }
}

impl<K: Into<String>, V: Into<Llsd>> From<HashMap<K, V>> for Llsd {
    fn from(llsd: HashMap<K, V>) -> Self {
        Llsd::Map(
            llsd.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

// Tuple support (2..=4) explicit implementations -------------------------------------------
impl<A: Into<Llsd>, B: Into<Llsd>> From<(A, B)> for Llsd {
    fn from(t: (A, B)) -> Self {
        let (a, b) = t;
        Llsd::Array(vec![a.into(), b.into()])
    }
}
impl<A, B> TryFrom<&Llsd> for (A, B)
where
    for<'x> A: TryFrom<&'x Llsd, Error = anyhow::Error>,
    for<'x> B: TryFrom<&'x Llsd, Error = anyhow::Error>,
{
    type Error = anyhow::Error;
    fn try_from(v: &Llsd) -> Result<Self> {
        if let Llsd::Array(a) = v {
            if a.len() == 2 {
                Ok((A::try_from(&a[0])?, B::try_from(&a[1])?))
            } else {
                Err(anyhow::Error::msg("Expected array of length 2"))
            }
        } else {
            Err(anyhow::Error::msg("Expected LLSD Array"))
        }
    }
}

impl<A: Into<Llsd>, B: Into<Llsd>, C: Into<Llsd>> From<(A, B, C)> for Llsd {
    fn from(t: (A, B, C)) -> Self {
        let (a, b, c) = t;
        Llsd::Array(vec![a.into(), b.into(), c.into()])
    }
}
impl<A, B, C> TryFrom<&Llsd> for (A, B, C)
where
    for<'x> A: TryFrom<&'x Llsd, Error = anyhow::Error>,
    for<'x> B: TryFrom<&'x Llsd, Error = anyhow::Error>,
    for<'x> C: TryFrom<&'x Llsd, Error = anyhow::Error>,
{
    type Error = anyhow::Error;
    fn try_from(v: &Llsd) -> Result<Self> {
        if let Llsd::Array(a) = v {
            if a.len() == 3 {
                Ok((
                    A::try_from(&a[0])?,
                    B::try_from(&a[1])?,
                    C::try_from(&a[2])?,
                ))
            } else {
                Err(anyhow::Error::msg("Expected array of length 3"))
            }
        } else {
            Err(anyhow::Error::msg("Expected LLSD Array"))
        }
    }
}

impl<A: Into<Llsd>, B: Into<Llsd>, C: Into<Llsd>, D: Into<Llsd>> From<(A, B, C, D)> for Llsd {
    fn from(t: (A, B, C, D)) -> Self {
        let (a, b, c, d) = t;
        Llsd::Array(vec![a.into(), b.into(), c.into(), d.into()])
    }
}
impl<A, B, C, D> TryFrom<&Llsd> for (A, B, C, D)
where
    for<'x> A: TryFrom<&'x Llsd, Error = anyhow::Error>,
    for<'x> B: TryFrom<&'x Llsd, Error = anyhow::Error>,
    for<'x> C: TryFrom<&'x Llsd, Error = anyhow::Error>,
    for<'x> D: TryFrom<&'x Llsd, Error = anyhow::Error>,
{
    type Error = anyhow::Error;
    fn try_from(v: &Llsd) -> Result<Self> {
        if let Llsd::Array(a) = v {
            if a.len() == 4 {
                Ok((
                    A::try_from(&a[0])?,
                    B::try_from(&a[1])?,
                    C::try_from(&a[2])?,
                    D::try_from(&a[3])?,
                ))
            } else {
                Err(anyhow::Error::msg("Expected array of length 4"))
            }
        } else {
            Err(anyhow::Error::msg("Expected LLSD Array"))
        }
    }
}

impl<K: Into<String>, V: Into<Llsd>> FromIterator<(K, V)> for Llsd {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Llsd::Map(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

impl TryFrom<&Llsd> for Uuid {
    type Error = anyhow::Error;

    fn try_from(llsd: &Llsd) -> Result<Self> {
        match llsd {
            Llsd::Uuid(llsd) => Ok(*llsd),
            Llsd::String(llsd) => Ok(Uuid::parse_str(llsd.as_str())?),
            _ => Err(anyhow::Error::msg("not a UUID")),
        }
    }
}

impl TryFrom<&Llsd> for Url {
    type Error = anyhow::Error;

    fn try_from(llsd: &Llsd) -> Result<Self> {
        match llsd {
            Llsd::Uri(uri) => Ok(uri.try_into()?),
            Llsd::String(llsd) => Ok(Url::parse(llsd.as_str())?),
            _ => Err(anyhow::Error::msg("not a URL")),
        }
    }
}

mod private {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for str {}
    impl Sealed for String {}
    impl<T> Sealed for &T where T: ?Sized + Sealed {}
}

pub trait Index: private::Sealed {
    fn index_into<'v>(&self, v: &'v Llsd) -> Option<&'v Llsd>;
    fn index_into_mut<'v>(&self, v: &'v mut Llsd) -> Option<&'v mut Llsd>;
    fn index_or_insert<'v>(&self, v: &'v mut Llsd) -> &'v mut Llsd;
}

impl<I> ops::Index<I> for Llsd
where
    I: Index,
{
    type Output = Llsd;
    fn index(&self, index: I) -> &Llsd {
        static NULL: Llsd = Llsd::Undefined;
        index.index_into(self).unwrap_or(&NULL)
    }
}

impl Index for usize {
    fn index_into<'v>(&self, v: &'v Llsd) -> Option<&'v Llsd> {
        match v {
            Llsd::Array(vec) => vec.get(*self),
            _ => None,
        }
    }
    fn index_into_mut<'v>(&self, v: &'v mut Llsd) -> Option<&'v mut Llsd> {
        match v {
            Llsd::Array(vec) => vec.get_mut(*self),
            _ => None,
        }
    }
    fn index_or_insert<'v>(&self, v: &'v mut Llsd) -> &'v mut Llsd {
        match v {
            Llsd::Array(vec) => {
                let len = vec.len();
                vec.get_mut(*self).unwrap_or_else(|| {
                    panic!(
                        "cannot access index {} of JSON array of length {}",
                        self, len
                    )
                })
            }
            _ => panic!("cannot access index {}", self),
        }
    }
}

impl Index for str {
    fn index_into<'v>(&self, v: &'v Llsd) -> Option<&'v Llsd> {
        match v {
            Llsd::Map(map) => map.get(self),
            _ => None,
        }
    }
    fn index_into_mut<'v>(&self, v: &'v mut Llsd) -> Option<&'v mut Llsd> {
        match v {
            Llsd::Map(map) => map.get_mut(self),
            _ => None,
        }
    }
    fn index_or_insert<'v>(&self, v: &'v mut Llsd) -> &'v mut Llsd {
        if let Llsd::Undefined = v {
            *v = Llsd::Map(HashMap::new());
        }
        match v {
            Llsd::Map(map) => map.entry(self.to_owned()).or_insert(Llsd::Undefined),
            _ => panic!("cannot access key {:?}", self),
        }
    }
}

impl<T> Index for &T
where
    T: ?Sized + Index,
{
    fn index_into<'v>(&self, v: &'v Llsd) -> Option<&'v Llsd> {
        (**self).index_into(v)
    }
    fn index_into_mut<'v>(&self, v: &'v mut Llsd) -> Option<&'v mut Llsd> {
        (**self).index_into_mut(v)
    }
    fn index_or_insert<'v>(&self, v: &'v mut Llsd) -> &'v mut Llsd {
        (**self).index_or_insert(v)
    }
}

impl Index for String {
    fn index_into<'v>(&self, v: &'v Llsd) -> Option<&'v Llsd> {
        self[..].index_into(v)
    }
    fn index_into_mut<'v>(&self, v: &'v mut Llsd) -> Option<&'v mut Llsd> {
        self[..].index_into_mut(v)
    }
    fn index_or_insert<'v>(&self, v: &'v mut Llsd) -> &'v mut Llsd {
        self[..].index_or_insert(v)
    }
}

impl<I> ops::IndexMut<I> for Llsd
where
    I: Index,
{
    fn index_mut(&mut self, index: I) -> &mut Llsd {
        index.index_or_insert(self)
    }
}

impl TryFrom<&Llsd> for bool {
    type Error = anyhow::Error;

    fn try_from(llsd: &Llsd) -> anyhow::Result<Self> {
        if let Some(value) = llsd.as_boolean() {
            Ok(*value)
        } else {
            Err(anyhow::Error::msg("Expected LLSD Boolean"))
        }
    }
}

impl TryFrom<&Llsd> for String {
    type Error = anyhow::Error;

    fn try_from(llsd: &Llsd) -> anyhow::Result<Self> {
        if let Some(value) = llsd.as_string() {
            Ok(value.clone())
        } else {
            Err(anyhow::Error::msg("Expected LLSD String"))
        }
    }
}

impl<T> TryFrom<&Llsd> for Vec<T>
where
    T: for<'a> TryFrom<&'a Llsd, Error = anyhow::Error>,
{
    type Error = anyhow::Error;

    fn try_from(llsd: &Llsd) -> anyhow::Result<Self> {
        if let Some(array) = llsd.as_array() {
            array.iter().map(|item| T::try_from(item)).collect()
        } else {
            Err(anyhow::Error::msg("Expected LLSD Array"))
        }
    }
}

impl<V> TryFrom<&Llsd> for HashMap<String, V>
where
    V: for<'a> TryFrom<&'a Llsd, Error = anyhow::Error>,
{
    type Error = anyhow::Error;

    fn try_from(llsd: &Llsd) -> anyhow::Result<Self> {
        if let Some(map) = llsd.as_map() {
            map.iter()
                .map(|(k, v)| Ok((k.clone(), V::try_from(v)?)))
                .collect()
        } else {
            Err(anyhow::Error::msg("Expected LLSD Map"))
        }
    }
}
