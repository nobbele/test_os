#![no_std]
#![no_main]
//#![feature(asm)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![allow(unused)]

use core::fmt::Write;

mod serial;
mod testing;
mod vga;

static HELLO: &[u8] = b"Hello World!";

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Welcome to my OS");

    #[cfg(test)]
    test_main();

    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    {
        let _guard = vga::WRITER
            .lock()
            .with_color(vga::Color::LightRed, vga::Color::LightCyan);
        println!("{}", info);
    }
    loop {}
}
