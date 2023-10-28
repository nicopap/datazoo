use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::meta::ParseNestedMeta;
use syn::spanned::Spanned;

use crate::config::ReadAttribute;

const NO_STRUCT_MSG: &'static str = "Flags derive macro only supports structs with field names.";
const NAMELESS_STRUCT_MSG: &'static str =
    "Flags derive macro only supports structs with field names. This struct doesn't have named fields.";

#[derive(Clone)]
struct Config {
    empty_tuple_reader: bool,
    flag_crate: syn::Path,
    packed_repr: Option<syn::Ident>,
    ident: syn::Ident,
    generics: syn::Generics,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            empty_tuple_reader: false,
            packed_repr: None,
            flag_crate: syn::parse_quote!(::dz_bitflags),
            ident: syn::Ident::new("nonsense", Span::call_site()),
            generics: Default::default(),
        }
    }
}
impl ReadAttribute for Config {
    const PATH: &'static str = "flags";
    fn read_attr(&mut self, meta: ParseNestedMeta) -> syn::Result<()> {
        match () {
            () if meta.path.is_ident("no_reader") => {
                self.empty_tuple_reader = true;
            }
            () if meta.path.is_ident("repr") => {
                let value = meta.value()?;
                self.packed_repr = Some(value.parse()?);
            }
            () if meta.path.is_ident("dz_bitflags_path") => {
                let value = meta.value()?;
                self.flag_crate = value.parse()?;
            }
            () => {
                let path = &meta.path;
                let ident = quote!(#path);
                let msg = format!("Unrecognized flag meta attribute: {ident}");
                return Err(meta.error(msg));
            }
        }
        Ok(())
    }
}

pub fn generate(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let span = input.span();
    let mut config = Config::default();
    config.read_attrs(&input.attrs)?;
    config.generics = input.generics;
    config.ident = input.ident;

    let syn::Data::Struct(data) = input.data else {
        return Err(syn::Error::new(span, NO_STRUCT_MSG));
    };
    let syn::Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new(span, NAMELESS_STRUCT_MSG));
    };
    let (reader, reader_impl) = if config.empty_tuple_reader {
        (quote!(()), None)
    } else {
        generate_reader(&config, fields)?
    };
}
fn generate_reader(
    config: &Config,
    input: &syn::FieldsNamed,
) -> syn::Result<(TokenStream, Option<TokenStream>)> {
    let path = &config.flag_crate;
    // TODO: add Bitsized bound to generic
    let generics = &config.generics;
    let ty_flags = &config.ident;
    let ty_reader = format_ident!("{ty_flags}Reader");
    // TODO: add links to relevant doc items.
    let body = quote! {
        /// Read individual fields from the packed representation of a `Flags` struct.
        #[derive(Clone, Copy)]
        #vis_flags struct #ty_reader(#ty_packed);

        impl #generics #path::FlagsReader<#ty_packed> for #ty_reader {
            fn from_packed(packed: #ty_packed) -> Self {
                Self(packed)
            }
        }
        impl struct #ty_reader {
            #(
                /// Accessor for
                #[doc = stringify!(#fields_name)]
                #[doc = ".\n"]
                #fields_attrs
                #vis_fields const fn #field_name(self) -> #ty_fields {
                    let packed = self.0;
                    #fields_accessor_impl
                }
            )*
        }
    };
    Ok((ty_reader.into(), Some(body)))
}
fn generate_flags(
    config: &Config,
    input: &syn::FieldsNamed,
) -> syn::Result<(TokenStream, Option<TokenStream>)> {
    let path = &config.flag_crate;
    // TODO: add Bitsized bound to generic
    let generics = &config.generics;
    let ty_flags = &config.ident;
    let body = quote! {
        impl #ty_flags {
            const _CHECK_SIZE: () = {
                assert!(<Self as #path::Flags>::Packed::BIT_SIZE >= <Self as #path::Bitsized>::BIT_SIZE);
            };
        }
        impl #generics #path::Flags for #ty_flags {
            type Packed = #ty_packed;
            type Reader = #ty_reader;

            fn from_packed(packed: Self::Packed) -> Self {
                Self { #( #field_name: #fields_accessor_impl ,)* }
            }
            fn into_packed(self) -> Self::Packed {
                todo!()
            }
        }
    };
    Ok((ty_flags.into(), Some(body)))
}
fn generate_accessor() -> TokenStream {
    quote! {{
        const THIS_OFFSET: u32 = 0 #(+ #preced_fields_size)*;
        const THIS_SIZE: u32 = <#ty_field as #path::Bitsized>::BIT_SIZE;
    }}
}
