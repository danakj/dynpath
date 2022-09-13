//! This crate provides a `#[dynpath()]` macro that can be placed on a `mod`
//! statement, and which points the module to a dynamic path.
//!
//! The primary purpose of this crate is to include bindgen-generated bindings
//! without an `include!()` statement. This allows for code completion and
//! cross-references.

#![deny(clippy::all)]

mod helpers;
mod parse;

use parse::WrapArgs;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, parse_quote, spanned::Spanned};

use crate::helpers::{get_modpath, Suffix};

const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

/// Attaches `#[path = ..]` to an existing mod dynamically.
///
/// NOTE: This macro requires you to use the nightly
/// [`proc_macro_hygiene`](https://github.com/rust-lang/rust/issues/54727) feature.
///
/// # Arguments
/// * First argument: The name of an environment variable to read the path from
/// * Second argument: The string to be concatenated to the first argument. If not
///   provided, it defaults to the name of the module the attribute is on.
///
/// # Example
/// ```ignore
/// #![feature(proc_macro_hygiene)]
/// // Turns into `#[path = "whatever/is/in/OUT_DIR/bindings.rs"]`.
/// #[dynpath("OUT_DIR")]
/// mod bindings;
/// ```
///
/// ```ignore
/// #![feature(proc_macro_hygiene)]
/// // Turns into `#[path = "whatever/is/in/OUT_DIR/generated/mod.rs"]`.
/// #[dynpath("OUT_DIR", "generated/mod.rs")]
/// mod bindings;
/// ```
#[proc_macro_attribute]
pub fn dynpath(
  attr: proc_macro::TokenStream,
  item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
  let attr = parse_macro_input!(attr as syn::AttributeArgs);
  let item = parse_macro_input!(item as syn::ItemMod);

  if !(attr.len() == 1 || attr.len() == 2) {
    return syn::Error::new(Span::call_site(), "Expected one or two arguments")
      .into_compile_error()
      .into();
  }

  let suffix = match attr.get(1) {
    None => Suffix::Mod(&item),
    Some(syn::NestedMeta::Lit(syn::Lit::Str(lit))) => Suffix::Literal(lit),
    _ => {
      return syn::Error::new(attr[1].span(), "Expected a string literal")
        .into_compile_error()
        .into();
    }
  };

  let modpath = match get_modpath(&attr[0], suffix) {
    Ok(s) => s,
    Err(e) => return e.into_compile_error().into(),
  };

  quote! {
    #[path = #modpath]
    #item
  }
  .into()
}

/// Wraps a dynpath statement such that it can be expanded without any nightly features.
///
/// # Example
/// ```ignore
/// // No nightly rust needed!
/// wrap! {
///   // Turns into `#[path = "whatever/is/in/OUT_DIR/bindings.rs"]`.
///   #[dynpath("OUT_DIR")]
///   mod bindings;
/// }
/// ``
#[proc_macro]
pub fn wrap(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
  let args = parse_macro_input!(args as WrapArgs);

  // Create a new module
  let mod_ident = args.mod_ident;
  let vis = args.vis;
  let item_mod: syn::ItemMod = parse_quote! {
    #vis mod #mod_ident;
  };

  // Process optional suffix argument
  let suffix = if let Some(ref l) = args.dynpath_args.suffix_lit {
    Suffix::Literal(l)
  } else {
    Suffix::Mod(&item_mod)
  };

  // Compute modpath
  let modpath = match get_modpath(&args.dynpath_args.env_var, suffix) {
    Ok(p) => p,
    Err(e) => return e.into_compile_error().into(),
  };

  let attrs = args.attrs;
  // Tokenify it
  quote! {
    #(#attrs)*
    #[path = #modpath]
    #item_mod
  }
  .into()
}
