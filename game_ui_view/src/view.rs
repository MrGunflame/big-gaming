use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, ExprBlock, Result, Token};

pub fn view(input: TokenStream) -> TokenStream {
    let node = parse_macro_input!(input as Node);
    node.into_token_stream().into()
}

#[derive(Clone, Debug)]
struct Node {
    name: Ident,
    children: Vec<Node>,
    attrs: Vec<(Ident, ExprBlock)>,
}

impl Parse for Node {
    fn parse(input: ParseStream) -> Result<Self> {
        // Head
        input.parse::<Token![<]>()?;
        let name = input.parse()?;

        // Attributes
        let mut attrs = Vec::new();
        while !input.peek(Token![>]) {
            let ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let expr = input.parse()?;

            attrs.push((ident, expr));
        }

        input.parse::<Token![>]>()?;

        // Children
        let mut children = Vec::new();
        if input.peek(Token![<]) && !input.peek2(Token![/]) {
            let node = input.parse()?;
            children.push(node);
        }

        // Tail
        input.parse::<Token![<]>()?;
        input.parse::<Token![/]>()?;

        let name2 = input.parse::<Ident>()?;
        assert_eq!(name, name2);

        input.parse::<Token![>]>()?;

        Ok(Self {
            name,
            children,
            attrs,
        })
    }
}

impl ToTokens for Node {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = self.name.clone();

        let attrs: TokenStream2 = self
            .attrs
            .iter()
            .map(|(id, expr)| {
                quote! {
                    props.#id =  #expr.into();
                }
            })
            .collect();

        let children: TokenStream2 = self
            .children
            .iter()
            .map(|child| {
                quote! {
                    #child
                }
            })
            .collect();

        tokens.extend(quote! {
            {
                let mut props = <#name as ::game_ui::widgets::Component>::Properties::default();
                #attrs

                let cx = <#name as ::game_ui::widgets::Component>::render(&cx, props);
                #children;

                cx
            }
        });
    }
}
