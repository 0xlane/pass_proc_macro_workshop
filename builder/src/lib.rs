use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod first;
use first::*;

// mod second;
// use second::*;

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // eprintln!("{:#?}", input);
    let expand = expand(input).unwrap_or_else(|err| err.to_compile_error());
    TokenStream::from(expand)
}

