use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn, Pat, PatIdent, Type};

pub fn component(attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as ItemFn);

    let mut props = HashMap::new();
    // Skip the Scope arg.
    for arg in item.sig.inputs.clone().into_iter().skip(1) {
        match arg {
            FnArg::Receiver(_) => panic!("invalid self in comp"),
            FnArg::Typed(pat) => {
                let ident = match *pat.pat {
                    Pat::Ident(id) => id.ident,
                    _ => panic!(),
                };

                props.insert(ident, pat.ty);
            }
        }
    }

    let vis = item.vis.clone();
    let ident = item.sig.ident.clone();
    let component_struct = quote! {
        #vis struct #ident;
    };

    let fields: TokenStream2 = props
        .iter()
        .map(|(id, ty)| {
            quote! {
                #id: #ty,
            }
        })
        .collect();

    let props_ident = Ident::new(&format!("{}Props", ident), Span::call_site());
    let props_struct = quote! {
        #[derive(Default)]
        struct #props_ident {
            #fields
        }
    };

    let call_args: TokenStream2 = props
        .iter()
        .map(|(id, _)| {
            quote! {
                #id,
            }
        })
        .collect();

    // Rename the function to a snake-case to avoid warnings.
    // Note that we can not simply disable the naming lint as that would also
    // disable the lint for the whole function body.
    item.sig.ident = Ident::new("render_component", Span::call_site());
    let fn_ident = item.sig.ident.clone();

    let component_impl = quote! {
        impl ::game_ui::widgets::Component for #ident {
            type Properties = #props_ident;

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
