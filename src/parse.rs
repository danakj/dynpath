use proc_macro2::Span;
use syn::Token;

pub struct DynpathArgs {
  pub env_var: syn::NestedMeta,
  pub suffix_lit: Option<syn::LitStr>,
}
impl syn::parse::Parse for DynpathArgs {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;
    syn::parenthesized!(content in input);
    let env_var = content.parse()?;
    content.parse::<Token![,]>()?;
    let suffix_lit = content.parse()?;
    Ok(Self {
      env_var,
      suffix_lit,
    })
  }
}

pub struct WrapArgs {
  pub vis: syn::Visibility,
  pub mod_ident: syn::Ident,
  pub dynpath_args: DynpathArgs,
  pub attrs: Vec<syn::Attribute>,
}
impl syn::parse::Parse for WrapArgs {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    // Parse and validate that attr statement is correct
    let (attr, attrs) = {
      let mut attrs = syn::Attribute::parse_outer(input)?;
      let idx = if let Some(idx) = attrs.iter().position(|a| {
        if let Some(ident) = a.path.get_ident() {
          ident.to_string() == crate::CRATE_NAME
        } else {
          false
        }
      }) {
        idx
      } else {
        return Err(syn::Error::new(
          Span::call_site(),
          "Expected a `dynpath` attribute",
        ));
      };
      let attr = attrs.remove(idx);
      (attr, attrs)
    };

    // Parse mod statement
    let vis: syn::Visibility = input.parse()?;
    input.parse::<Token![mod]>()?;
    let mod_ident: syn::Ident = input.parse()?;
    input.parse::<Token![;]>()?;

    // Parse arguments to dynpath
    let dynpath_args: DynpathArgs = syn::parse2(attr.tokens)?;

    Ok(Self {
      vis,
      mod_ident,
      dynpath_args,
      attrs,
    })
  }
}
