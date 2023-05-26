use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::Parse;
use syn::{parse_macro_input, FnArg, ItemFn, Lifetime, Pat, Type, Visibility};

pub fn component(attr: TokenStream, input: TokenStream) -> TokenStream {
    parse_macro_input!(attr as Empty);

    let item = parse_macro_input!(input as ItemFn);

    let mut props = Vec::new();
    // Skip the Scope arg.
    for arg in item.sig.inputs.clone().into_iter().skip(1) {
        match arg {
            FnArg::Receiver(_) => panic!("invalid self in comp"),
            FnArg::Typed(pat) => {
                let ident = match *pat.pat {
                    Pat::Ident(id) => id.ident,
                    _ => panic!(),
                };

                props.push((ident, pat.ty));
            }
        }
    }

    if item.sig.asyncness.is_some() {
        panic!("component functions cannot be async");
    }

    if item.sig.unsafety.is_some() {
        panic!("component functions cannot be unsafe");
    }

    if item.sig.constness.is_some() {
        panic!("component functions cannot be const");
    }

    let props = Properties::new(props);

    let vis = item.vis.clone();
    let ident = item.sig.ident.clone();
    let component_struct = expand_component_struct(&vis, ident.clone(), &props);

    let props_ident = Ident::new(&format!("{}Props", ident), Span::call_site());
    let props_struct = expand_property_struct(&vis, props_ident.clone(), &props);

    let props_sig = expand_property_struct_sig(props_ident, &props);

    let call_args: TokenStream2 = props
        .props
        .iter()
        .map(|p| {
            let id = p.ident.clone();

            quote! {
                props.#id,
            }
        })
        .collect();

    let mut item = item;
    // Rename the function to a snake-case to avoid warnings.
    // Note that we can not simply disable the naming lint as that would also
    // disable the lint for the whole function body.
    item.sig.ident = Ident::new("render_component", Span::call_site());
    let fn_ident = item.sig.ident.clone();

    let lifetimes = props.struct_lifetimes();

    let component_impl = quote! {
        impl<#lifetimes> ::game_ui::widgets::Component for #ident<#lifetimes> {
            type Properties = #props_sig;

            fn render(cx: &::game_ui::reactive::Scope, props: Self::Properties) -> ::game_ui::reactive::Scope {
                // Always inline as this is the single call site.
                #[inline]
                #item

                #fn_ident(cx, #call_args)
            }

        }
    };

    quote! {
        #component_struct
        #props_struct
        #component_impl
    }
    .into()
}

struct Empty;

impl Parse for Empty {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            Ok(Self)
        } else {
            Err(input.error("macro does not take any attributes"))
        }
    }
}

struct Properties {
    props: Vec<Property>,
}

struct Property {
    ident: Ident,
    lifetime: Option<Lifetime>,
    ty: Type,
}

impl Property {
    fn expand_struct_field(&self) -> TokenStream2 {
        let ident = self.ident.clone();
        let lifetime = self.lifetime.clone();
        let ty = self.ty.clone();

        if let Some(lifetime) = lifetime {
            quote! {
                pub #ident: &#lifetime #ty,
            }
        } else {
            quote! {
                pub #ident: #ty,
            }
        }
    }
}

impl Properties {
    fn new(input: Vec<(Ident, Box<Type>)>) -> Self {
        let mut props = Vec::new();
        for (index, (ident, ty)) in input.into_iter().enumerate() {
            let prop = match *ty {
                Type::Reference(rf) => {
                    let lifetime = match rf.lifetime {
                        Some(lt) => lt,
                        None => {
                            let id = format!("'l{}", index);
                            Lifetime::new(&id, Span::call_site())
                        }
                    };

                    Property {
                        ident,
                        lifetime: Some(lifetime),
                        ty: *rf.elem,
                    }
                }
                ty => Property {
                    ident,
                    lifetime: None,
                    ty,
                },
            };

            props.push(prop);
        }

        Self { props }
    }

    fn struct_lifetimes(&self) -> TokenStream2 {
        self.props
            .iter()
            .map(|prop| {
                let lifetime = prop.lifetime.clone();

                if let Some(lifetime) = lifetime {
                    quote! { #lifetime, }
                } else {
                    quote! {}
                }
            })
            .collect()
    }
}

fn expand_component_struct(vis: &Visibility, ident: Ident, props: &Properties) -> TokenStream2 {
    let lifetimes = props.struct_lifetimes();

    let fields: TokenStream2 = props
        .props
        .iter()
        .enumerate()
        .map(|(index, p)| {
            if let Some(lifetime) = p.lifetime.clone() {
                let ident = Ident::new(&format!("_l{}", index), Span::call_site());

                quote! {
                    #ident: ::core::marker::PhantomData<fn() -> &#lifetime ()>,
                }
            } else {
                quote! {}
            }
        })
        .collect();

    quote! {
        #vis struct #ident<#lifetimes> {
            #fields
        }
    }
}

fn expand_property_struct(vis: &Visibility, ident: Ident, props: &Properties) -> TokenStream2 {
    let sig = expand_property_struct_sig(ident, props);

    let fields: TokenStream2 = props
        .props
        .iter()
        .map(|prop| prop.expand_struct_field())
        .collect();

    quote! {
        #vis struct #sig {
            #fields
        }
    }
}

fn expand_property_struct_sig(ident: Ident, props: &Properties) -> TokenStream2 {
    let lifetimes = props.struct_lifetimes();

    quote! {
        #ident<#lifetimes>
    }
}
