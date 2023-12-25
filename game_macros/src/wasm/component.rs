use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields, Index};

pub fn encode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    Input::new(input).expand_encode_trait_impl().into()
}

pub fn decode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    Input::new(input).expand_decode_trait_impl().into()
}

pub fn component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    Input::new(input).expand_component_trait_impl().into()
}

struct Input {
    fields: Vec<Field>,
    ident: Ident,
    kind: InputKind,
}

impl Input {
    fn new(input: DeriveInput) -> Self {
        let (fields, kind) = match input.data {
            Data::Struct(data) => match data.fields {
                Fields::Named(fields) => {
                    (fields.named.iter().cloned().collect(), InputKind::Struct)
                }
                Fields::Unnamed(fields) => (
                    fields.unnamed.iter().cloned().collect(),
                    InputKind::TupleStruct,
                ),
                Fields::Unit => (Vec::new(), InputKind::TupleStruct),
            },
            Data::Enum(_) | Data::Union(_) => todo!(),
        };

        Self {
            fields,
            ident: input.ident,
            kind,
        }
    }

    fn expand_encode_trait_impl(&self) -> TokenStream2 {
        let ident = self.ident.clone();

        let fields = self
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| match &field.ident {
                Some(ident) => {
                    quote! {
                        ::game_wasm::components::Encode::encode(&self.#ident, &mut buf);
                    }
                }
                None => {
                    let index = Index {
                        index: index as u32,
                        span: Span::call_site(),
                    };

                    quote! {
                        ::game_wasm::components::Encode::encode(&self.#index, &mut buf);
                    }
                }
            })
            .collect::<TokenStream2>();

        quote! {
            impl ::game_wasm::components::Encode for #ident {
                fn encode<__B>(&self, mut buf: __B)
                where
                    __B: ::game_wasm::components::BufMut,
                {
                    #fields
                }
            }
        }
    }

    fn expand_decode_trait_impl(&self) -> TokenStream2 {
        let ident = self.ident.clone();

        let fields = self
            .fields
            .iter()
            .map(|field| {
                let ty = field.ty.clone();

                match &field.ident {
                    Some(ident) => {
                        quote! {
                            #ident: <#ty as ::game_wasm::components::Decode>::decode(&mut buf)?,
                        }
                    }
                    None => {
                        quote! {
                            <#ty as ::game_wasm::components::Decode>::decode(&mut buf)?
                        }
                    }
                }
            })
            .collect::<TokenStream2>();

        let decode_fn_body = match self.kind {
            InputKind::Struct => quote! {
                Ok(Self {
                    #fields
                })
            },
            InputKind::TupleStruct => quote! {
                Ok(Self(#fields))
            },
        };

        quote! {
            impl ::game_wasm::components::Decode for #ident {
                type Error = ::game_wasm::components::DecodeError;

                fn decode<__B>(mut buf: __B) -> ::core::result::Result<Self, Self::Error>
                where
                    __B: ::game_wasm::components::Buf,
                {
                    #decode_fn_body
                }
            }
        }
    }

    fn expand_component_trait_impl(&self) -> TokenStream2 {
        let ident = self.ident.clone();

        let component = quote! {
            impl ::game_wasm::components::Component for #ident {}
        };

        vec![
            self.expand_encode_trait_impl(),
            self.expand_decode_trait_impl(),
            component,
        ]
        .into_iter()
        .collect()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum InputKind {
    Struct,
    TupleStruct,
}
