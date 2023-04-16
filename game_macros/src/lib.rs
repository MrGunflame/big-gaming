use proc_macro::TokenStream;

#[cfg(feature = "net")]
mod net;

mod wasm;

mod proto;

#[proc_macro_derive(Encode)]
pub fn encode(input: TokenStream) -> TokenStream {
    proto::encode(input)
}

// == net ==

#[proc_macro_derive(net__encode)]
#[allow(non_snake_case)]
#[cfg(feature = "net")]
pub fn net__encode(input: TokenStream) -> TokenStream {
    net::encode(input)
}

#[proc_macro_derive(net__decode)]
#[allow(non_snake_case)]
#[cfg(feature = "net")]
pub fn net__decode(input: TokenStream) -> TokenStream {
    net::decode(input)
}

// == wasm ==

#[proc_macro_attribute]
#[allow(non_snake_case)]
#[cfg(feature = "wasm")]
pub fn wasm__event_on_action(attr: TokenStream, input: TokenStream) -> TokenStream {
    wasm::events::on_action(attr, input)
}
