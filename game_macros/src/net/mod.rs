//! Macros for the net crate

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields};

pub fn encode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    Input::new(input).encode().into()
}

pub fn decode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    Input::new(input).decode().into()
}

struct Input {
    ident: Ident,
    fields: Vec<Field>,
}

impl Input {
    fn new(input: DeriveInput) -> Self {
        let fields = match input.data {
            Data::Struct(data) => match data.fields {
                Fields::Named(fields) => fields.named.iter().cloned().collect(),
                Fields::Unnamed(fields) => fields.unnamed.iter().cloned().collect(),
                Fields::Unit => Vec::new(),
            },
            Data::Enum(_) => panic!("enums are unsupported"),
            Data::Union(_) => panic!("unions are unsupported"),
        };

        Self {
            ident: input.ident,
            fields,
        }
    }

    fn encode(&self) -> TokenStream2 {
        let ident = self.ident.clone();

        let fields = self
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let ident = match &field.ident {
                    Some(ident) => ident.clone(),
                    None => Ident::new(&index.to_string(), Span::call_site()),
                };

                quote! {
                    self.#ident.encode(&mut buf)?;
                }
            })
            .collect::<TokenStream2>();

        quote! {
            impl Encode for #ident {
                type Error = Infallible;

                fn encode<__B>(&self, mut buf: __B) -> ::core::result::Result<(), Self::Error>
                where
                    __B: ::bytes::BufMut,
                {
                    #fields
                    Ok(())
                }
            }
        }
    }

    fn decode(&self) -> TokenStream2 {
        let ident = self.ident.clone();

        let fields = self
            .fields
            .iter()
            .map(|field| {
                let ident = field.ident.clone().unwrap();
                let ty = field.ty.clone();

                quote! {
                    #ident: #ty::decode(&mut buf)?,
                }
            })
            .collect::<TokenStream2>();

        quote! {
            impl Decode for #ident {
                type Error = Error;

                fn decode<__B>(mut buf: __B) -> ::core::result::Result<Self, Self::Error>
                where
                    __B: ::bytes::Buf,
                {
                    Ok(Self {
                        #fields
                    })
                }
            }
        }
    }
}
