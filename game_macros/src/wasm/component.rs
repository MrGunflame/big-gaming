use proc_macro::{Punct, TokenStream};
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::punctuated::Punctuated;
use syn::token::{As, Colon, Const, Gt, Lt, PathSep};
use syn::{
    parse_macro_input, AngleBracketedGenericArguments, ConstParam, Data, DeriveInput, Field,
    Fields, GenericParam, Generics, Index, Path, PathArguments, PathSegment, QSelf, Token,
    TraitBound, TraitBoundModifier, Type, TypeParam, TypeParamBound, TypePath,
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
                Fields::Unit => (Vec::new(), InputKind::TupleStruct),
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
                            ident: Ident::new("components", Span::call_site()),
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
            impl #generics ::game_wasm::components::Encode for #ident #generic_idents
            where
                #where_bounds
            {
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
                            ident: Ident::new("components", Span::call_site()),
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
            impl #generics ::game_wasm::components::Decode for #ident #generic_idents
            where
                #where_bounds
                #error_bounds
            {
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
            _ => todo!(),
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
        match param {
            GenericParam::Type(param) => {
                param.bounds.push(TypeParamBound::Trait(bound.clone()));
            }
            _ => (),
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
