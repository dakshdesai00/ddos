#![no_std]
#![no_main]

use core::arch::global_asm;
global_asm!(include_str!("boot.s"));

mod console;
mod font;
mod framebuffer;
mod mailbox;
mod uart;

use core::fmt::Write;
use core::panic::PanicInfo;

#[unsafe(no_mangle)]
pub extern "C" fn _main() -> ! {
    let mut uart = uart::Uart::new();
    let _ = writeln!(uart, "\nSystem Starting...");

    match framebuffer::FrameBuffer::new() {
        Ok(fb) => {
            let mut console = console::Console::new(fb);

            console.set_color(0xFF00FF00);
            let _ = writeln!(console, "Welcome to DDOS");

            console.set_color(0xFF00FFFF);
            let _ = writeln!(console, "Version 1.0");

            console.set_color(0xFFFFFF00);
            let _ = writeln!(console, "--------------------------");
            console.set_color(0xFFFFFFFF);
            let _ = write!(console, "\n> ");

            loop {
                let byte = uart.read_byte();
                match byte {
                    b'\r' => {
                        let _ = write!(console, "\n> ");
                    }
                    127 | 8 => {
                        console.backspace();
                    }
                    _ => {
                        console.set_color(0xFF0000FF);

                        let c = byte as char;
                        let _ = write!(console, "{}", c);

                        console.set_color(0xFFFFFFFF);
                    }
                }
            }
        }
        Err(code) => {
            let _ = writeln!(uart, "HDMI Failed. Code: 0x{:X}", code);
            loop {}
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
