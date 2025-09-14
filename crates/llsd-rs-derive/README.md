# llsd-rs-derive

Procedural macro companion crate for [`llsd-rs`](../llsd-rs). Provides derive macros to convert between user structs and the `Llsd` data model.

## Provided Derives

- `#[derive(LlsdFrom)]` – implements `TryFrom<&Llsd>` for your type.
- `#[derive(LlsdInto)]` – implements `From<T>` for `Llsd`.
- `#[derive(LlsdFromTo)]` – convenience combo (`LlsdFrom` + `LlsdInto`).

## Supported Field / Container Attributes

```rust
#[llsd(rename = "fieldName")]            // override individual field name
#[llsd(rename_all = "case")]              // container-wide: snake_case | kebab-case | camelCase | PascalCase | SCREAMING_SNAKE_CASE
#[llsd(default)]                           // use Default::default()
#[llsd(default = path::to_fn)]             // use custom function -> T
#[llsd(skip)]                              // skip for both serialize & deserialize
#[llsd(skip_serializing)]                  // only skip on into-LLSD
#[llsd(skip_deserializing)]                // only skip on from-LLSD
#[llsd(flatten)]                           // merge nested map fields (simple implementation)
#[llsd(deny_unknown_fields)]               // error on unrecognized input keys
#[llsd(with = module_path)]                // custom per-field (de)serializer: serialize(&T)->Llsd, deserialize(&Llsd)->Result<T>
```

## Example

```rust
use llsd_rs::{Llsd, LlsdFromTo};

#[derive(LlsdFromTo, Debug, PartialEq, Clone)]
#[llsd(rename_all = "camelCase", deny_unknown_fields)]
struct WidgetConfig {
    id: u32,
    #[llsd(default)] name: Option<String>,
    #[llsd(rename = "dataMap")] data: std::collections::HashMap<String, Item>,
    tuple: (i32, String),
}

#[derive(LlsdFromTo, Debug, PartialEq, Clone)]
struct Item { value: i32 }

let mut map = std::collections::HashMap::new();
map.insert("first".into(), Item { value: 10 });
let cfg = WidgetConfig { id: 1, name: None, data: map, tuple: (5, "hi".into()) };
let l: Llsd = cfg.clone().into();
let back: WidgetConfig = WidgetConfig::try_from(&l).unwrap();
assert_eq!(cfg, back);
```

## Feature Flag

The derives are only available when you enable the `derive` feature on the core crate:

```toml
[dependencies]
llsd-rs = { version = "0.1", features = ["derive"] }
```

## Error Handling

All generated `TryFrom<&Llsd>` impls return `anyhow::Error`. Future improvements may introduce a dedicated error type with richer context.

## Limitations / Roadmap

- `flatten` is shallow (expects nested value -> Map) – deeper merge semantics planned.
- Error messages are minimal (no field path yet).
- Generic type parameters require manual trait bounds currently.
- Additional collection / tuple arities may be added.

See the workspace `DERIVE.md` for extended design notes.

## Testing

Tests live in the core crate (`tests/derive_basic.rs`) exercising field attributes, tuples, maps, custom `with` serialization.
Run all tests:

```bash
cargo test --all-features
```

## License

LGPL-2.1. See `LICENCE`.
