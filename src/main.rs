#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(bare_metal::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{boxed::Box, rc::Rc, string::String, vec, vec::Vec};
use bare_metal::{
    interrupts::{clear_screen, disable_cursor},
    print, println,
};
use bootloader::{entry_point, BootInfo};
use core::arch::x86_64::__cpuid;
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
    println!("[*] ok\n[ ] no");

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

fn get_cpu_vendor() -> Vec<u8> {
    let cpuid = unsafe { __cpuid(0) };
    let mut vendor = [0u8; 12];
    vendor[0..4].copy_from_slice(&cpuid.ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&cpuid.edx.to_le_bytes());
    vendor[8..12].copy_from_slice(&cpuid.ecx.to_le_bytes());
    vendor.to_vec()
}
