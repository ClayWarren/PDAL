//! Procedural macros for the PDAL Rust C ABI.

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Error, Expr, Ident, ItemFn, Result, Token};

#[derive(Default)]
struct ExportArgs {
    fallback: Option<Expr>,
}

impl Parse for ExportArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.is_empty() {
            return Ok(Self::default());
        }

        let key: Ident = input.parse()?;
        if key != "fallback" {
            return Err(Error::new(key.span(), "expected `fallback = <expression>`"));
        }
        input.parse::<Token![=]>()?;
        let fallback = input.parse()?;

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        if !input.is_empty() {
            return Err(input.error("unexpected ffi_export argument"));
        }

        Ok(Self {
            fallback: Some(fallback),
        })
    }
}

/// Export an `extern "C"` function behind PDAL's Rust panic boundary.
///
/// The default panic result is `Default::default()`. Functions whose C ABI
/// contract uses another error sentinel can specify it explicitly:
///
/// ```ignore
/// #[ffi_export(fallback = u64::MAX)]
/// pub unsafe extern "C" fn example() -> u64
/// {
///     // ...
/// }
/// ```
///
/// The consuming crate must provide `crate::error::ffi_catch`.
#[proc_macro_attribute]
pub fn ffi_export(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as ExportArgs);
    let function = parse_macro_input!(item as ItemFn);

    match expand_ffi_export(args, function) {
        Ok(expanded) => expanded.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand_ffi_export(args: ExportArgs, function: ItemFn) -> Result<proc_macro2::TokenStream> {
    let signature = &function.sig;
    let abi = signature
        .abi
        .as_ref()
        .and_then(|abi| abi.name.as_ref())
        .map(|name| name.value());
    if abi.as_deref() != Some("C") {
        return Err(Error::new_spanned(
            &function.sig,
            "ffi_export requires an `extern \"C\"` function",
        ));
    }
    if !signature.generics.params.is_empty() || signature.generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            &function.sig.generics,
            "ffi_export does not support generic functions",
        ));
    }
    if signature.variadic.is_some() {
        return Err(Error::new_spanned(
            &function.sig,
            "ffi_export does not support variadic functions",
        ));
    }
    if signature.asyncness.is_some() || signature.constness.is_some() {
        return Err(Error::new_spanned(
            &function.sig,
            "ffi_export does not support async or const functions",
        ));
    }

    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = function;
    let fallback = args
        .fallback
        .map(|fallback| quote!(#fallback))
        .unwrap_or_else(|| quote!(::core::default::Default::default()));
    let guarded_body = if sig.unsafety.is_some() {
        quote! {
            crate::error::ffi_catch(#fallback, || {
                #[allow(unused_unsafe)]
                unsafe #block
            })
        }
    } else {
        quote! {
            crate::error::ffi_catch(#fallback, || #block)
        }
    };

    Ok(quote! {
        #(#attrs)*
        #[no_mangle]
        #vis #sig
        {
            #guarded_body
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{expand_ffi_export, ExportArgs};
    use syn::parse_quote;

    #[test]
    fn rejects_non_c_abi() {
        let error = expand_ffi_export(
            ExportArgs::default(),
            parse_quote!(
                pub fn example() -> bool {
                    true
                }
            ),
        )
        .expect_err("Rust ABI must be rejected");

        assert!(error.to_string().contains("extern \"C\""));
    }

    #[test]
    fn rejects_generic_functions() {
        let error = expand_ffi_export(
            ExportArgs::default(),
            parse_quote!(
                pub extern "C" fn example<T>() {}
            ),
        )
        .expect_err("generic exports must be rejected");

        assert!(error.to_string().contains("generic"));
    }
}
