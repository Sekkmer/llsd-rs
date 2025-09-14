# llsd-rs Workspace

This workspace contains the core LLSD implementation crate and its optional derive (proc-macro) companion.

## Crates

| Crate | Path | Description |
|-------|------|-------------|
| `llsd-rs` | `crates/llsd-rs` | Core data types (`Llsd`), parsing & serialization for Binary, XML, Notation, XML-RPC. |
| `llsd-rs-derive` | `crates/llsd-rs-derive` | Procedural macros: `#[derive(LlsdFrom)]`, `#[derive(LlsdInto)]`, `#[derive(LlsdFromTo)]`. |

## Quick Start

Add the core crate:

```toml
[dependencies]
llsd-rs = { version = "0.1", features = ["derive"] } # enable derives (optional)
```

Minimal example:

```rust
use llsd_rs::Llsd;

let value = Llsd::map()
    .insert("name", "Alice").unwrap()
    .insert("age", 30u32).unwrap();
assert!(matches!(value, Llsd::Map(_)));
```

With derives:

```rust
use llsd_rs::{LlsdFromTo, Llsd};

#[derive(LlsdFromTo, Debug, PartialEq, Clone)]
struct User { #[llsd(rename = "userId")] id: u32, #[llsd(default)] name: Option<String> }

let u = User { id: 7, name: None };
let l: Llsd = u.clone().into();
let back: User = User::try_from(&l).unwrap();
assert_eq!(u, back);
```

## Repository Layout

```
crates/
  llsd-rs/          # core library
  llsd-rs-derive/   # proc-macro derives (optional)
examples/           # usage examples
DERIVE.md           # extended derive attribute documentation / roadmap
```

## Development

Run all tests (including doctests & derive tests):

```bash
cargo test --all-features
```

Lint (deny warnings):

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## License

LGPL-2.1. See `LICENCE`.

## Status / Roadmap (High Level)

- Core formats: Binary, Notation, XML, XML-RPC (done)
- Derive macros: basic field attributes (done)
- Future: richer error context, improved `flatten` semantics, expanded tuple & collection support in derives

Contributions and issues welcome.
