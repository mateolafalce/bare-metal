#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(bare_metal::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bare_metal::{
    interrupts::{clear_screen, disable_cursor},
    println,
};
use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use bare_metal::allocator;
    use bare_metal::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    bare_metal::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    disable_cursor();
    clear_screen();
    println!("[*] Reboot\n[ ] Shutdown");

    bare_metal::hlt_loop();
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    bare_metal::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info)
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
