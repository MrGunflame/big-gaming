use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, ItemFn, Result};

pub fn on_action(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as Args);
    let input = parse_macro_input!(input as ItemFn);

    let ident = input.sig.ident;
    let inputs = input.sig.inputs;

    let block = input.block;

    // We force the function signature to be correct
    // by asserting that the signature fits into the
    // function.
    let fn_assert = quote! {
        #[inline(always)]
        fn __assert_fn_signature_on_action(f: unsafe extern "C" fn(u64, u64)) {}
    };

    let expanded = quote! {
        #[no_mangle]
        pub extern "C" fn on_action(#inputs) {
            {
                #fn_assert
                __assert_fn_signature_on_action(#ident);
            }

            #block
        }
    };

    TokenStream::from(expanded)
}

struct Args {}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {})
    }
}
