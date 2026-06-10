#![no_std]
#![no_main]

extern crate alloc;
use alloc::vec::Vec;

use core::panic::PanicInfo;
use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};
use morse_core::{MorseBitstream, MorseCharacter};
use morse_macro::morse;
use wasm_bindgen::prelude::wasm_bindgen;

#[global_allocator]
static ALLOCATOR: AssumeSingleThreaded<FreeListAllocator> = unsafe {
    AssumeSingleThreaded::new(FreeListAllocator::new())
};

/// Expose our conversion trigger to Trunk / JS.
#[wasm_bindgen]
pub fn process_macro_stream() -> Vec<u8> {
    let macro_stream: MorseBitstream = morse!("hello from wasm!");
    let mut output = Vec::new();

    macro_stream.fold_characters((), |_, morse_char| {
        match morse_char {
            MorseCharacter::Character(c) => {
                output.push(c.to_ascii_lowercase() as u8);
            }
            MorseCharacter::IntraWord => {
                output.push(b' ');
            }
            MorseCharacter::Stop => {
                output.push(b'!');
            }
        }
    });

    output
}

#[wasm_bindgen(start)]
pub fn main() {}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}