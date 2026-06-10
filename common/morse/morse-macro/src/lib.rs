extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};
use morse_core::encode_text;

#[proc_macro]
pub fn morse(input: TokenStream) -> TokenStream {
    // Parse input as a string literal
    let input_str = parse_macro_input!(input as LitStr).value();

    // Call shared, unified logic directly at compile time!
    let bitstream = encode_text(&input_str);

    let bytes = bitstream.bytes;
    let symbol_count = bitstream.symbol_count;

    // Expand into a structural literal matching MorseBitstream layout
    let expanded = quote! {
        morse_core::MorseBitstream {
            bytes: [ #(#bytes),* ],
            symbol_count: #symbol_count,
        }
    };

    TokenStream::from(expanded)
}