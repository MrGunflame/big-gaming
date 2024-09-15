use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, ItemFn};

pub fn crash_handler_main(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);

    let fn_call = if let Some(token) = input.sig.unsafety {
        quote_spanned! {
            token.span() =>
            compile_error!("function cannot be unsafe")
        }
    } else {
        let ident = &input.sig.ident;
        quote! {
            // SAFETY: Since we are exporting as the `main` function we
            // can guarantee that we are not in a multithreaded
            // environment and no signal handlers have been changed yet.
            unsafe { ::game_crash_handler::run(#ident) }
        }
    };

    TokenStream::from(quote! {
        fn main() -> ::std::process::ExitCode {
            #input
            #fn_call
        }
    })
}
