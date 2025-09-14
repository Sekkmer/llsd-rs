//! Derive macro user-facing documentation (implementation in `llsd-rs-derive`).
//!
//! Enable via Cargo feature:
//! ```toml
//! llsd-rs = { version = "<latest>", features = ["derive"] }
//! ```
//!
//! Then import the macros (re-exported when the `derive` feature is active):
//! ```rust
//! # #[cfg(feature = "derive")]
//! use llsd_rs::{LlsdFrom, LlsdInto, LlsdFromTo};
//! ```
//!
//! Basic example (feature-gated so doctest always compiles):
//! ```rust
//! # #[cfg(feature = "derive")]
//! # use llsd_rs::{LlsdFromTo, Llsd};
//! # #[cfg(feature = "derive")]
//! #[derive(LlsdFromTo)]
//! struct User {
//!     #[llsd(rename = "userId")] id: u32,
//!     #[llsd(default)] name: Option<String>,
//! }
//! # #[cfg(feature = "derive")]
//! # fn demo() {
//! #     let u = User { id: 1, name: None };
//! #     let _l: Llsd = u.into();
//! # }
//! # #[cfg(not(feature = "derive"))]
//! # fn demo() {}
//! ```
//!
//! Supported (currently implemented) attributes:
//! - `#[llsd(rename = "fieldName")]`
//! - `#[llsd(rename_all = "case")]` on the container: snake_case | kebab-case | camelCase | PascalCase | SCREAMING_SNAKE_CASE
//! - `#[llsd(default)]` or `#[llsd(default = "path::to_fn")]`
//! - `#[llsd(skip)]`, `#[llsd(skip_serializing)]`, `#[llsd(skip_deserializing)]`
//! - `#[llsd(flatten)]` (experimental; simple merge of nested map fields)
//! - `#[llsd(deny_unknown_fields)]`
//!
//! Notes / Limitations:
//! - `with = "path"` attribute is parsed but not yet applied.
//! - `flatten` currently only works for fields whose LLSD form is a Map.
//! - Generic structs: bounds are not auto-inferred; add them manually if needed.
//! - Error messages are basic; future improvement will add per-field context.
//!
//! All macro expansion code lives in the `llsd-rs-derive` crate so this
//! module is intentionally minimal.

#[allow(dead_code)]
pub struct _DeriveDocs;
