use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((foreground as u8) | (background as u8) << 4)
    }
}

impl Default for ColorCode {
    fn default() -> Self {
        ColorCode::new(Color::White, Color::Black)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
pub struct VGABuffer([[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT]);

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        position: (0, 0),
        color_code: ColorCode::default(),
        buffer: unsafe { &mut *(0xb8000 as *mut _) },
    });
}

pub struct ColorGuard {
    initial: ColorCode,
}

impl Drop for ColorGuard {
    fn drop(&mut self) {
        WRITER.lock().color_code = self.initial;
    }
}

pub struct Writer {
    position: (usize, usize),
    color_code: ColorCode,
    buffer: &'static mut VGABuffer,
}

impl Writer {
    pub fn with_color(&mut self, fg: Color, bg: Color) -> ColorGuard {
        let initial = self.color_code;
        self.color_code = ColorCode::new(fg, bg);
        ColorGuard { initial }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                let color_code = self.color_code;
                self.buffer.0[self.position.1][self.position.0].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.position.0 += 1;

                if self.position.0 >= BUFFER_WIDTH {
                    self.new_line();
                }
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: ColorCode::default(),
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.0[row][col].write(blank);
        }
    }

    fn new_line(&mut self) {
        if self.position.1 >= BUFFER_HEIGHT - 1 {
            for row in 1..BUFFER_HEIGHT {
                for col in 0..BUFFER_WIDTH {
                    let character = self.buffer.0[row][col].read();
                    self.buffer.0[row - 1][col].write(character);
                }
            }
            self.position.1 = BUFFER_HEIGHT - 2;
        }

        self.position.0 = 0;
        self.position.1 += 1;
        self.clear_row(self.position.1);
    }
}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    x86_64::instructions::interrupts::without_interrupts(|| WRITER.lock().write_fmt(args).unwrap());
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use core::fmt::Write;

    #[test_case]
    fn test_color_guard() {
        assert_eq!(WRITER.lock().color_code, ColorCode::default());
        {
            let _guard = WRITER.lock().with_color(Color::Red, Color::White);
            assert_eq!(
                WRITER.lock().color_code,
                ColorCode::new(Color::Red, Color::White)
            );
        }
        assert_eq!(WRITER.lock().color_code, ColorCode::default());
    }

    #[test_case]
    fn test_println() {
        println!("Hello World!");
    }

    #[test_case]
    fn test_println_output() {
        x86_64::instructions::interrupts::without_interrupts(|| {
            let mut writer = WRITER.lock();
            writeln!(writer).expect("writeln failed");
            let start_row = writer.position.1;
            let s = "Foo Bar";
            writeln!(writer, "{}", s).expect("writeln failed");
            for (idx, c) in s.chars().enumerate() {
                let screen_char = writer.buffer.0[start_row][idx].read().ascii_character;
                assert_eq!(c, screen_char as char);
            }
        });
    }

    #[test_case]
    fn test_println_many() {
        x86_64::instructions::interrupts::without_interrupts(|| {
            let mut writer = WRITER.lock();
            for i in 0..200 {
                writeln!(writer, "Hello World! {}", i).expect("writeln failed");
            }
            let last_text = "Hello World! 199";
            for (idx, c) in last_text.chars().enumerate() {
                let vga_char = writer.buffer.0[BUFFER_HEIGHT - 2][idx]
                    .read()
                    .ascii_character;
                assert_eq!(c, vga_char as char);
            }
        });
    }
}
