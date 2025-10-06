#![no_std]

extern crate alloc;

use alloc::boxed::Box;

use defmt::info;
use embedded_alloc::LlffHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

pub fn run() {
    init_heap();

    let mut meh = Box::new(42);

    *meh = 69;

    info!("heap test: {}", *meh);

    drop(meh);
}

#[allow(static_mut_refs)]
fn init_heap() {
    use core::mem::MaybeUninit;
    const HEAP_SIZE: usize = 1024;
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
}
