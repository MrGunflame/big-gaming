use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{parse_macro_input, parse_str, ItemFn, Result, Type};

macro_rules! define_action {
    ($($ident:ident => $inputs:expr),*$(,)?) => {
        $(
            pub fn $ident(attr: TokenStream, input: TokenStream) -> TokenStream {
                let mut inputs = vec![];
                for elem in $inputs {
                    match parse_str(elem) {
                        Ok(t) => inputs.push(t),
                        Err(err) => {
                            panic!("internal error: {}", err);
                        }
                    };
                }

                expand_event_attr(attr, input, inputs)
            }
        )*
    };
}

define_action! {
    on_action => ["u64", "u64"],
    on_collision => ["u64", "u64"],
}

fn expand_event_attr<T>(attr: TokenStream, input: TokenStream, inputs: T) -> TokenStream
where
    T: IntoIterator<Item = Type>,
{
    parse_macro_input!(attr as EmptyArgs);
    let input = parse_macro_input!(input as ItemFn);

    let expanded = expand_extern(input, Punctuated::from_iter(inputs));

    TokenStream::from(expanded)
}

fn expand_extern(func: ItemFn, inputs: Punctuated<Type, Comma>) -> TokenStream2 {
    let assertion = expand_assertion_block(func.sig.ident.clone(), inputs);

    let ident = func.sig.ident;
    let inputs = func.sig.inputs;
    let output = func.sig.output;
    let block = func.block;

    quote! {
        #[no_mangle]
        pub extern "C" fn #ident(#inputs) #output {
            #assertion
            #block
        }
    }
}

fn expand_assertion_block(ident: Ident, inputs: Punctuated<Type, Comma>) -> TokenStream2 {
    quote! {
        {
            #[inline(always)]
            fn __assert_fn_signature(_: unsafe extern "C" fn(#inputs)) {}
            __assert_fn_signature(#ident);
        }
    }
}

struct EmptyArgs;

impl Parse for EmptyArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        if !input.is_empty() {
            input
                .span()
                .unwrap()
                .error("cannot add arguments to event macro")
                .emit();
        }

        Ok(Self {})
    }
}
