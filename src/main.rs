#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(test_os::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![allow(unused)]

extern crate alloc;

use alloc::boxed::Box;
use bootloader::{entry_point, BootInfo};
use core::fmt::Write;
use test_os::{
    allocator,
    memory::{self, BootInfoFrameAllocator},
    println, vga,
};
use x86_64::{structures::paging::Translate, VirtAddr};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Welcome to my OS");

    test_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    #[cfg(test)]
    test_main();

    println!("Goodbye!");

    test_os::halt_loop();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use test_os::serial_println;

    {
        let _guard = vga::WRITER
            .lock()
            .with_color(vga::Color::LightRed, vga::Color::LightCyan);
        println!("{}", info);
        serial_println!("{}", info);
    }
    test_os::halt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    test_os::test_panic(info)
}
