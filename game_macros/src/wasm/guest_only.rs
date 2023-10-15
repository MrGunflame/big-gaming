use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, ForeignItemFn, LitStr};

pub fn guest_only(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ForeignItemFn);

    let cfg_predicate = quote! { target_arch = "wasm32" };

    let binding_function = expand_binding_function(item.clone());
    let stub_function = expand_stub_function(item);

    quote! {
        #[cfg(#cfg_predicate)]
        #binding_function
        #[cfg(not(#cfg_predicate))]
        #stub_function
    }
    .into()
}

fn expand_binding_function(item: ForeignItemFn) -> TokenStream2 {
    quote! {
        #[link(wasm_import_module = "host")]
        extern "C" {
            #item
        }
    }
}

fn expand_stub_function(item: ForeignItemFn) -> TokenStream2 {
    let vis = item.vis;
    let ident = item.sig.ident;
    let inputs = item.sig.inputs;
    let output = item.sig.output;
    let attrs = item.attrs;

    let panic_msg = LitStr::new(
        &format!("`{}` is not implemented on this target", ident),
        Span::call_site(),
    );

    let attrs: TokenStream2 = attrs.iter().map(ToTokens::to_token_stream).collect();

    quote! {
        #attrs
        #[allow(unused_variables)]
        #vis unsafe extern "C" fn #ident(#inputs) #output {
            ::core::panic!(#panic_msg);
        }
    }
}
