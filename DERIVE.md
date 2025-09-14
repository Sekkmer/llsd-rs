# LLSD Derive Functionality

This document describes the derive functionality added to the `llsd-rs` crate, which provides utilities for automatically implementing `From<Llsd>` and `TryFrom<&Llsd>` traits for custom structs.

## Overview

The `derive.rs` module provides a comprehensive framework for implementing derive macros that can automatically generate serialization and deserialization code for LLSD (Linden Lab Structured Data) format. While this module is designed to work with procedural macros, the example shows how the patterns can be applied manually.

## Features

### Field Renaming

Support for various case conventions:
- `snake_case` (default Rust convention)
- `camelCase` (JavaScript/JSON convention)
- `PascalCase` (C# convention)
- `kebab-case` (HTML/CSS convention)
- `SCREAMING_SNAKE_CASE` (constant convention)
- `lowercase` (all lowercase)
- `UPPERCASE` (all uppercase)

### Supported Attributes

#### Container Attributes
- `#[llsd(rename_all = "case")]` - Apply a case convention to all field names
- `#[llsd(deny_unknown_fields)]` - Reject LLSD with unknown fields during deserialization

#### Field Attributes
- `#[llsd(rename = "name")]` - Use a custom name for this field in LLSD
- `#[llsd(skip)]` - Skip this field during both serialization and deserialization
- `#[llsd(skip_serializing)]` - Skip this field during serialization only
- `#[llsd(skip_deserializing)]` - Skip this field during deserialization only
- `#[llsd(default)]` - Use `Default::default()` if field is missing during deserialization
- `#[llsd(default = "path")]` - Use a custom function to provide default values
- `#[llsd(flatten)]` - Flatten this field's contents into the parent structure
- `#[llsd(with = "module")]` - Use custom serialization/deserialization functions

### Supported Field Types

The derive functionality supports all basic Rust types that can be converted to/from LLSD:

- **Primitives**: `bool`, `i32`, `f64`, `String`
- **Optional**: `Option<T>` where `T` implements the required traits
- **Collections**: `Vec<T>`, `HashMap<String, T>`
- **LLSD types**: `Uuid`, `Uri`, `DateTime<Utc>`
- **Custom structs**: Any struct that implements the required traits

## Usage Examples

### Basic Struct with Default Field Names

```rust
#[derive(Debug, Clone, PartialEq)]
struct Person {
    pub first_name: String,
    pub last_name: String,
    pub age: u32,
    pub email: Option<String>,
}

// Manually implement the traits (in a real proc macro, this would be generated)
impl TryFrom<&Llsd> for Person { /* ... */ }
impl From<&Person> for Llsd { /* ... */ }
impl From<Person> for Llsd { /* ... */ }
```

### Struct with camelCase Field Names

```rust
#[derive(Debug, Clone, PartialEq)]
// In a real proc macro: #[derive(LlsdFromTo)]
// #[llsd(rename_all = "camelCase")]
struct UserProfile {
    pub user_id: u64,        // becomes "userId" in LLSD
    pub display_name: String, // becomes "displayName" in LLSD
    pub is_active: bool,     // becomes "isActive" in LLSD
}
```

### Struct with Custom Field Names and Skipped Fields

```rust
#[derive(Debug, Clone, PartialEq)]
// In a real proc macro: #[derive(LlsdFromTo)]
struct Employee {
    #[llsd(rename = "employeeId")]
    pub id: u64,
    
    pub name: String,
    
    #[llsd(skip)]
    pub internal_notes: String, // Not included in LLSD
    
    #[llsd(default)]
    pub department: Option<String>, // Uses Default::default() if missing
}
```

## Case Conversion Examples

The derive system automatically handles field name transformations:

| Rust Field Name | snake_case | camelCase | PascalCase | kebab-case | SCREAMING_SNAKE_CASE |
|-----------------|------------|-----------|------------|------------|---------------------|
| `user_id`       | `user_id`  | `userId`  | `UserId`   | `user-id`  | `USER_ID`          |
| `firstName`     | `first_name` | `firstName` | `FirstName` | `first-name` | `FIRST_NAME`     |
| `isActive`      | `is_active` | `isActive` | `IsActive` | `is-active` | `IS_ACTIVE`      |

## Error Handling

The derive functionality provides comprehensive error handling:

- **Missing required fields**: Clear error messages indicating which field is missing
- **Type conversion errors**: Descriptive errors when LLSD values can't be converted to expected types
- **Unknown fields**: Optional strict mode to reject LLSD with unexpected fields
- **Invalid LLSD structure**: Errors when expecting a Map but receiving other LLSD types

## Implementation Details

### Generated `TryFrom<&Llsd>` Implementation

The derive macro generates implementations that:
1. Check if the LLSD value is a Map
2. Extract each field from the map using the appropriate name (after case conversion)
3. Convert each value to the target type using existing `TryFrom` implementations
4. Handle optional fields and default values appropriately
5. Construct the target struct

### Generated `From<T>` for `Llsd` Implementation

The derive macro generates implementations that:
1. Create a new HashMap for the LLSD Map
2. Insert each non-skipped field using the appropriate name (after case conversion)
3. Convert each field value to LLSD using existing `From` implementations
4. Handle optional fields (skip `None` values)
5. Return the resulting LLSD Map

## Testing

The derive functionality includes comprehensive tests covering:
- Basic struct conversion (round-trip)
- Optional field handling
- Field name transformations
- Error cases (missing fields, wrong types)
- Edge cases (empty structs, nested structures)

To run the tests:

```bash
cargo test --example derive_usage
```

## Future Enhancements

The derive system is designed to be extensible and could support:
- Enum serialization/deserialization
- Tuple struct support
- Custom validation attributes
- Conditional compilation attributes
- Nested attribute inheritance
- Performance optimizations for large structures

## Integration with Procedural Macros

This module provides the foundation for implementing procedural derive macros. The functions `derive_from_llsd`, `derive_into_llsd`, and `derive_llsd_from_to` can be used by procedural macros to generate the appropriate `TokenStream` for the derived implementations.

A complete procedural macro implementation would typically:
1. Parse the input `DeriveInput` using `syn`
2. Call the appropriate derive function from this module
3. Return the generated `TokenStream`
4. Handle compilation errors gracefully

## See Also

- [examples/derive_usage.rs](../examples/derive_usage.rs) - Complete working example
- [src/lib.rs](../src/lib.rs) - Core LLSD types and basic trait implementations
- [llsd-rs-derive/](../llsd-rs-derive/) - Existing procedural macro implementation
