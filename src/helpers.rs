use syn::spanned::Spanned;

pub enum Suffix<'a> {
  Mod(&'a syn::ItemMod),
  Literal(&'a syn::LitStr),
}

pub fn get_modpath(env_var: &syn::NestedMeta, suffix: Suffix) -> syn::Result<String> {
  let env_var = match env_var {
    syn::NestedMeta::Lit(syn::Lit::Str(lit)) => lit.value(),
    _ => {
      return Err(syn::Error::new(
        env_var.span(),
        "Argument should be the name of an environment variable, e.g. `\"OUT_DIR\"`",
      ));
    }
  };
  let prefix = std::env::var(&env_var)
    .unwrap_or_else(|_| panic!("The \"{}\" environment variable is not set", &env_var));

  let modpath = std::path::PathBuf::from(prefix);
  let modpath = match suffix {
    Suffix::Mod(m) => {
      let modname = m.ident.to_string();
      modpath.join(format!("{}.rs", modname))
    }
    Suffix::Literal(l) => modpath.join(l.value()),
  };

  Ok(format!("{}", modpath.display()))
}
