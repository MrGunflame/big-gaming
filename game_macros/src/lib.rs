#![feature(proc_macro_diagnostic)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

use proc_macro::TokenStream;

#[cfg(feature = "net")]
mod net;

mod wasm;

mod proto;

macro_rules! reexport_attribute_macro {
    ($($ident:ident => $dst:path),*$(,)?) => {
        $(
            #[proc_macro_attribute]
            #[allow(non_snake_case)]
            pub fn $ident(attr: TokenStream, input: TokenStream) -> TokenStream {
                $dst(attr, input)
            }
        )*
    };
}

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

#[cfg(feature = "wasm")]
reexport_attribute_macro! {
    wasm__event_on_init => wasm::events::on_init,
    wasm__event_on_action => wasm::events::on_action,
    wasm__event_on_collision => wasm::events::on_collision,
    wasm__event_on_equip => wasm::events::on_equip,
    wasm__event_on_unequip => wasm::events::on_unequip,
    wasm__event_on_cell_load => wasm::events::on_cell_load,
    wasm__event_on_cell_unload => wasm::events::on_cell_unload,
}

#[cfg(feature = "wasm")]
#[proc_macro_attribute]
pub fn guest_only(attr: TokenStream, input: TokenStream) -> TokenStream {
    crate::wasm::guest_only::guest_only(attr, input)
}
