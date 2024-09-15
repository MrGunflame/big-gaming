use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::punctuated::Punctuated;
use syn::token::{Gt, Lt, PathSep};
use syn::{
    parse_macro_input, Data, DeriveInput, Field, Fields, GenericParam, Generics, Index, Path,
    PathArguments, PathSegment, TraitBound, TraitBoundModifier, TypeParam, TypeParamBound,
};

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
    generics: Generics,
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
                Fields::Unit => (Vec::new(), InputKind::Unit),
            },
            Data::Enum(_) | Data::Union(_) => todo!(),
        };

        Self {
            fields,
            ident: input.ident,
            kind,
            generics: input.generics,
        }
    }

    fn expand_encode_trait_impl(&self) -> TokenStream2 {
        let ident = self.ident.clone();

        let generic_idents = expand_generic_idents(&self.generics);
        let mut generics = self.generics.clone();
        add_trait_bound(
            &mut generics,
            TraitBound {
                paren_token: None,
                modifier: TraitBoundModifier::None,
                lifetimes: None,
                path: Path {
                    leading_colon: Some(PathSep::default()),
                    segments: Punctuated::from_iter([
                        PathSegment {
                            ident: Ident::new("game_wasm", Span::call_site()),
                            arguments: PathArguments::None,
                        },
                        PathSegment {
                            ident: Ident::new("encoding", Span::call_site()),
                            arguments: PathArguments::None,
                        },
                        PathSegment {
                            ident: Ident::new("Encode", Span::call_site()),
                            arguments: PathArguments::None,
                        },
                    ]),
                },
            },
        );

        let where_bounds = self
            .generics
            .clone()
            .where_clause
            .map(|clause| clause.predicates)
            .unwrap_or_default();

        let fields = self
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| match &field.ident {
                Some(ident) => {
                    quote! {
                        ::game_wasm::encoding::Encode::encode(&self.#ident, &mut writer);
                    }
                }
                None => {
                    let index = Index {
                        index: index as u32,
                        span: Span::call_site(),
                    };

                    quote! {
                        ::game_wasm::encoding::Encode::encode(&self.#index, &mut writer);
                    }
                }
            })
            .collect::<TokenStream2>();

        quote! {
            impl #generics ::game_wasm::encoding::Encode for #ident #generic_idents
            where
                #where_bounds
            {
                fn encode<__W>(&self, mut writer: __W)
                where
                    __W: ::game_wasm::encoding::Writer,
                {
                    #fields
                }
            }
        }
    }

    fn expand_decode_trait_impl(&self) -> TokenStream2 {
        let ident = self.ident.clone();

        let generic_idents = expand_generic_idents(&self.generics);
        let mut generics = self.generics.clone();
        add_trait_bound(
            &mut generics,
            TraitBound {
                paren_token: None,
                modifier: TraitBoundModifier::None,
                lifetimes: None,
                path: Path {
                    leading_colon: Some(PathSep::default()),
                    segments: Punctuated::from_iter([
                        PathSegment {
                            ident: Ident::new("game_wasm", Span::call_site()),
                            arguments: PathArguments::None,
                        },
                        PathSegment {
                            ident: Ident::new("encoding", Span::call_site()),
                            arguments: PathArguments::None,
                        },
                        PathSegment {
                            ident: Ident::new("Decode", Span::call_site()),
                            arguments: PathArguments::None,
                        },
                    ]),
                },
            },
        );

        let error_bounds = require_error_conversion(&generics);
        let where_bounds = self
            .generics
            .clone()
            .where_clause
            .map(|clause| clause.predicates)
            .unwrap_or_default();

        let fields = self
            .fields
            .iter()
            .map(|field| {
                let ty = field.ty.clone();

                match &field.ident {
                    Some(ident) => {
                        quote! {
                            #ident: <#ty as ::game_wasm::encoding::Decode>::decode(&mut reader)?,
                        }
                    }
                    None => {
                        quote! {
                            <#ty as ::game_wasm::encoding::Decode>::decode(&mut reader)?
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
            InputKind::Unit => quote! {
                Ok(Self)
            },
        };

        quote! {
            impl #generics ::game_wasm::encoding::Decode for #ident #generic_idents
            where
                #where_bounds
                #error_bounds
            {
                type Error = ::game_wasm::encoding::DecodeError;

                fn decode<__R>(mut reader: __R) -> ::core::result::Result<Self, Self::Error>
                where
                    __R: ::game_wasm::encoding::Reader,
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
    Unit,
}

fn expand_generic_idents(generics: &Generics) -> TokenStream2 {
    let params = generics
        .params
        .iter()
        .map(|param| match param {
            GenericParam::Type(param) => GenericParam::Type(TypeParam {
                attrs: Vec::new(),
                ident: param.ident.clone(),
                colon_token: None,
                bounds: Punctuated::new(),
                eq_token: None,
                default: None,
            }),
            GenericParam::Lifetime(_) => panic!("lifetimes are not supported"),
            GenericParam::Const(param) => GenericParam::Type(TypeParam {
                attrs: Vec::new(),
                ident: param.ident.clone(),
                colon_token: None,
                bounds: Punctuated::new(),
                eq_token: None,
                default: None,
            }),
        })
        .collect();

    Generics {
        lt_token: Some(Lt::default()),
        gt_token: Some(Gt::default()),
        where_clause: None,
        params,
    }
    .to_token_stream()
}

fn add_trait_bound(generics: &mut Generics, bound: TraitBound) {
    for param in &mut generics.params {
        if let GenericParam::Type(param) = param {
            param.bounds.push(TypeParamBound::Trait(bound.clone()));
        }
    }
}

fn require_error_conversion(generics: &Generics) -> TokenStream2 {
    let idents: Vec<_> = generics
        .params
        .iter()
        .filter_map(|param| match param {
            GenericParam::Type(param) => Some(param.ident.clone()),
            _ => None,
        })
        .collect();

    idents
        .iter()
        .map(|ident| {
            quote! {
                DecodeError: From<<#ident as ::game_wasm::components::Decode>::Error>,
            }
        })
        .collect()
}
