#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

mod component;
mod view;

use proc_macro::TokenStream;

#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    view::view(input)
}

#[proc_macro_attribute]
pub fn component(attr: TokenStream, input: TokenStream) -> TokenStream {
    component::component(attr, input)
}
