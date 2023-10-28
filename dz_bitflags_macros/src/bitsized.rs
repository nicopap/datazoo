use std::num::NonZeroU32;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::meta::ParseNestedMeta;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Token;

use crate::config::ReadAttribute;

const UNION_MSG: &'static str =
    "Bitsized derive macro only supports single-field structs and C-like enums.";
const SINGLE_FIELD_MSG: &'static str =
    "Bitsized derive macro only supports single-field structs, this struct has zero or more than one field.";
const EMPTY_ENUM_MSG: &'static str =
    "Bitsized derive macro do not support empty enums, as an empty enum cannot be instanciated";

#[derive(Clone)]
struct Config {
    explicit_size: Option<NonZeroU32>,
    flag_crate: syn::Path,
    ident: syn::Ident,
    generics: syn::Generics,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            explicit_size: None,
            flag_crate: syn::parse_quote!(::dz_bitflags),
            ident: syn::Ident::new("nonsense", Span::call_site()),
            generics: Default::default(),
        }
    }
}
impl ReadAttribute for Config {
    const PATH: &'static str = "bitsized";
    fn read_attr(&mut self, meta: ParseNestedMeta) -> syn::Result<()> {
        match () {
            () if meta.path.is_ident("max") => {
                let value = meta.value()?;
                let max: syn::LitInt = value.parse()?;
                self.explicit_size = Some(max.base10_parse::<NonZeroU32>()?);
            }
            () if meta.path.is_ident("dz_bitflags_path") => {
                let value = meta.value()?;
                self.flag_crate = value.parse()?;
            }
            () => {
                let path = &meta.path;
                let ident = quote!(#path);
                let msg = format!("Unrecognized bitsized meta attribute: {ident}");
                return Err(meta.error(msg));
            }
        }
        Ok(())
    }
}
impl Config {
    fn bit_size(&self, default_size: impl FnOnce() -> TokenStream) -> TokenStream {
        if let Some(value) = self.explicit_size {
            let value = value.get();
            quote!(#value)
        } else {
            default_size()
        }
    }

    fn bit_size_fits(&self, bit_size: u32) -> bool {
        self.explicit_size
            .iter()
            .all(|explicit| bit_size <= explicit.get())
    }
}

pub fn generate(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let span = input.span();
    let mut config = Config::default();
    config.read_attrs(&input.attrs)?;
    config.generics = input.generics;
    config.ident = input.ident;

    match input.data {
        syn::Data::Struct(struct_data) => generate_struct(config, struct_data),
        syn::Data::Enum(enum_data) => generate_enum(config, enum_data),
        syn::Data::Union(_) => Err(syn::Error::new(span, UNION_MSG)),
    }
}
fn struct_unique_field(fields: &syn::Fields) -> syn::Result<(Option<&syn::Ident>, &syn::Type)> {
    match fields {
        syn::Fields::Named(fields) if fields.named.len() != 1 => {
            let field = fields.named.first().unwrap();
            Ok((field.ident.as_ref(), &field.ty))
        }
        syn::Fields::Unnamed(fields) if fields.unnamed.len() != 1 => {
            Ok((None, &fields.unnamed.first().unwrap().ty))
        }
        _ => Err(syn::Error::new(fields.span(), SINGLE_FIELD_MSG)),
    }
}
fn generate_struct(config: Config, input: syn::DataStruct) -> syn::Result<TokenStream> {
    let path = &config.flag_crate;
    let generics = &config.generics;
    let ty_name = &config.ident;
    let (_field, ty_field) = struct_unique_field(&input.fields)?;
    let bit_size = config.bit_size(|| quote!(<#ty_field as #path::Bitsized>::BIT_SIZE));
    Ok(quote! {
        impl #generics #path::Bitsized for #ty_name {
            const BIT_SIZE: u32 = #bit_size;
        }
    })
}
fn enum_bit_size(variants: &Punctuated<syn::Variant, Token![,]>) -> syn::Result<u32> {
    let variant_count = variants.len();
    if variant_count == 0 {
        return Err(syn::Error::new(variants.span(), EMPTY_ENUM_MSG));
    }
    Ok(variant_count.ilog2() + 1)
}
fn generate_enum(config: Config, input: syn::DataEnum) -> syn::Result<TokenStream> {
    let path = &config.flag_crate;
    // TODO: add Bitsized bound to generic
    let generics = &config.generics;
    let ty_name = &config.ident;
    let bit_size = enum_bit_size(&input.variants)?;
    if !config.bit_size_fits(bit_size) {
        let explicit = config.explicit_size.unwrap();
        let msg = format!(
            "Cannot be represented in less than {bit_size} bits, explicit size is {explicit}."
        );
        return Err(syn::Error::new(input.variants.span(), msg));
    }
    let bit_size = config.bit_size(|| quote!(#bit_size));
    Ok(quote! {
        impl #generics #path::Bitsized for #ty_name {
            const BIT_SIZE: u32 = #bit_size;
        }
    })
}
