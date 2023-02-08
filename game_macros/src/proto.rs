use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields};

pub fn encode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let fields = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named.iter().cloned().collect(),
            Fields::Unnamed(fields) => fields.unnamed.iter().cloned().collect(),
            Fields::Unit => Vec::new(),
        },
        Data::Enum(_) => unimplemented!(),
        Data::Union(_) => unimplemented!(),
    };

    let ident = input.ident.clone();

    let size_impl = expand_size_impl(&fields);
    let encode_impl = expand_encode_impl(&fields);

    TokenStream::from(quote! {
        unsafe impl Encode for #ident {
            #[inline]
            fn size(&self) -> usize {
                #size_impl
            }

            #[inline]
            unsafe fn encode(&self, mut buf: *mut u8) {
                unsafe {
                    #encode_impl
                }
            }
        }
    })
}

fn expand_size_impl(fields: &[Field]) -> TokenStream2 {
    let fields = fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let ident = match &field.ident {
                Some(ident) => ident.clone(),
                None => Ident::new(&index.to_string(), Span::call_site()),
            };

            quote! {
                res += self.#ident.size();
            }
        })
        .collect::<TokenStream2>();

    quote! {
        let mut res: usize = 0;
        #fields
        res
    }
}

fn expand_encode_impl(fields: &[Field]) -> TokenStream2 {
    fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let ident = match &field.ident {
                Some(ident) => ident.clone(),
                None => Ident::new(&index.to_string(), Span::call_site()),
            };

            // Encode the field and move the pointer forward.
            quote! {
                self.#ident.encode(buf);
                buf = buf.add(self.#ident.size());
            }
        })
        .collect::<TokenStream2>()
}
