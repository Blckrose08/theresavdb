# Fuzzing Foundation for TheresavDB

TheresavDB uses property-based testing and `cargo-fuzz` (libFuzzer) to ensure zero-trust decoding across all binary formats.

## Targets:
- `fuzz_page_decode`: Fuzzes arbitrary 16 KiB buffers through `Page::from_bytes` ensuring zero panics or undefined behavior.
- `fuzz_tuple_decode`: Fuzzes arbitrary byte buffers through `Tuple::deserialize`.
- `fuzz_types_decode`: Fuzzes binary deserialization across all supported SQL data types.
- `fuzz_jsonb_decode`: Fuzzes arbitrary byte sequences into `JsonbValue::decode`.

## Running Fuzzers:
```bash
cargo install cargo-fuzz
cargo fuzz run fuzz_page_decode
```
