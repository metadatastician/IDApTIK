//! A `Deserialize` derive that Creusot can translate.
//!
//! Creusot 0.13 cannot translate serde's own derived `Deserialize`, and the
//! failure is crate-wide: one derive anywhere stops the whole crate being
//! verified. Two separate mechanisms in serde's generated code hit walls in
//! the translator --- the `const FIELDS: &[&str]` field-name table raises
//! `Unsupported constant expression`, and the error path raises
//! `forbidden dyn type: dyn serde::de::Expected`. The second is structural,
//! not a passing bug: Creusot's `DYN_COMPATIBLE` set
//! (`creusot/src/backend/clone_map/elaborator.rs`) is a hardcoded list of six
//! traits --- `Any`, `Error`, `Debug`, `Display`, `fmt::Write`, `io::Write` ---
//! and `serde::de::Expected` is not one of them, so no version bump inside
//! 0.13 can admit it.
//!
//! Measured, not assumed (three-way control in an isolation crate): a crate
//! with no serde derive translates cleanly, a crate deriving only `Serialize`
//! translates cleanly, and a crate deriving only `Deserialize` fails. So only
//! `Deserialize` needs replacing, which is why every `#[serde(...)]` helper
//! attribute in `idaptik-core` is left exactly as it is --- `Serialize` still
//! reads them.
//!
//! This crate supplies a derive of the same name that emits a `#[trusted]`
//! impl with an `unreachable!()` body. Under `cfg(creusot)` --- and only then,
//! since `cfg(creusot)` is set by `creusot-rustc` and by nothing else --- the
//! crate root re-exports this derive in place of serde's. The shipped build
//! never sees it.
//!
//! What that buys, precisely: the trait impl still *exists*, so every
//! `serde_json::from_str` call site type-checks unchanged and nothing has to
//! be cfg'd out of the call graph. What it costs, equally precisely: a
//! `#[trusted]` function carries no postcondition, so Creusot treats a
//! deserialized value as *unconstrained* rather than as satisfying anything.
//! That is sound --- proofs downstream may assume less, never more --- but it
//! does mean deserialization itself is unverified. That is one row in
//! `crates/CREUSOT-PROOF-DEBT.tsv`, not one row per type.
//!
//! The failure mode is loud, which is the point: a module that reaches for
//! `use serde::Deserialize` directly instead of the crate-root re-export puts
//! serde's derive back in scope and breaks the Creusot build immediately.

use proc_macro::TokenStream;
use quote::quote;

/// Stand-in for `serde::Deserialize`'s derive under `cfg(creusot)`.
///
/// `attributes(serde)` is required even though nothing here reads them: the
/// derive must still *consume* `#[serde(...)]` helper attributes on the type
/// it is applied to, or they become unknown attributes and fail to compile.
#[proc_macro_derive(Deserialize, attributes(serde))]
pub fn deserialize(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = match syn::parse(input) {
        Ok(ast) => ast,
        Err(e) => return e.to_compile_error().into(),
    };
    let name = &ast.ident;

    // Borrow the type's own generics and add the `'de` the trait needs. The
    // stub body uses none of the type parameters, so no `T: Deserialize<'de>`
    // bounds are needed -- unlike serde's derive, which must recurse.
    let mut generics = ast.generics.clone();
    generics.params.insert(0, syn::parse_quote!('de));
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let (_, ty_generics, _) = ast.generics.split_for_impl();

    quote! {
        impl #impl_generics ::serde::Deserialize<'de> for #name #ty_generics #where_clause {
            #[::creusot_std::macros::trusted]
            fn deserialize<__D>(_deserializer: __D) -> ::core::result::Result<Self, __D::Error>
            where
                __D: ::serde::Deserializer<'de>,
            {
                unreachable!(
                    "creusot-serde-shim: deserialization is not translated under cfg(creusot)"
                )
            }
        }
    }
    .into()
}
