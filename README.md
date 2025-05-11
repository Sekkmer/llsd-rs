# llsd-rs

A native Rust implementation of Linden Lab’s LLSD (Linden Lab Structured Data) serialization formats.

## Features

- Encode and decode LLSD types:
  - Undefined, Boolean, Integer, Real, String
  - URI, UUID, Date, Binary
  - Array and Map structures
- Support for LLSD **Binary**, **XML**, **National**, and **XML-RPC** serialization
- Zero-copy & allocation-minimal where possible
- Inspired by and compatible with the Second Life viewer’s LLSD codebase

## Installation

Execute `cargo add llsd-rs`
