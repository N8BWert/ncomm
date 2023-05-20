extern crate quote;
extern crate syn;

use proc_macro::TokenStream;

use syn::{DeriveInput, parse_macro_input};
use quote::quote;

#[proc_macro_derive(Request)]
pub fn request_macro_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let expanded = quote! {
        impl Request for #name {}
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(Response)]
pub fn response_macro_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let expanded = quote! {
        impl Response for #name {}
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(UpdateMessage)]
pub fn update_message_macro_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let expanded = quote! {
        impl UpdateMessage for #name {}
    };

    TokenStream::from(expanded)
}