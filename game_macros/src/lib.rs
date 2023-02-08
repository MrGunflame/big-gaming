use proc_macro::TokenStream;

mod proto;

#[proc_macro_derive(Encode)]
pub fn encode(input: TokenStream) -> TokenStream {
    proto::encode(input)
}
