# dynpath

[Documentation](https://docs.rs/dynpath)

This crate provides a `#[dynpath()]` macro that can be placed on a `mod`
statement, and which points the module to a dynamic path.

The primary purpose of this crate is to include bindgen-generated bindings
without an `include!()` statement. This allows for code completion and
cross-references.

The macro takes a single parameter which is the name of an environment variable
to read the path from, and it appends the module name and `.rs` extension onto
the contents of the variable.

## Example
```rs
// Turns into `#[path = "whatever/is/in/OUT_DIR/bindings.rs"]`.
#[dynpath("OUT_DIR")]
mod bindings;
```

# License
This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  https://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

## Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in dynpath by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

