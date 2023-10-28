#![warn(clippy::pedantic, clippy::nursery, missing_docs)]
#![allow(
    clippy::use_self,
    clippy::module_name_repetitions,
    clippy::redundant_pub_crate
)]

use proc_macro::TokenStream as TokenStream1;
use syn::parse_macro_input;

mod bitsized;
mod config;
mod flags;

#[proc_macro_derive(Bitsized, attributes(bitsized))]
pub fn derive_bitsized(item: TokenStream1) -> TokenStream1 {
    match bitsized::generate(parse_macro_input!(item as syn::DeriveInput)) {
        Ok(stream) => stream.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro_derive(Flags, attributes(flags))]
pub fn derive_flags(item: TokenStream1) -> TokenStream1 {
    match flags::generate(parse_macro_input!(item as syn::DeriveInput)) {
        Ok(stream) => stream.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
