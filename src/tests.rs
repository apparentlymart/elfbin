use super::*;
use std::io::{Cursor, Result};

// Borrowed from https://crates.io/crates/assert_hex, under MIT license.
macro_rules! assert_eq_hex {
    ($left:expr, $right:expr $(,)?) => ({
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    // The reborrows below are intentional. Without them, the stack slot for the
                    // borrow is initialized even before the values are compared, leading to a
                    // noticeable slow down.
                    panic!(r#"assertion failed: `(left == right)`
  left: `{:#x?}`,
 right: `{:#x?}`"#, &*left_val, &*right_val)
                }
            }
        }
    });
    ($left:expr, $right:expr, $($arg:tt)+) => ({
        match (&($left), &($right)) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    // The reborrows below are intentional. Without them, the stack slot for the
                    // borrow is initialized even before the values are compared, leading to a
                    // noticeable slow down.
                    panic!(r#"assertion failed: `(left == right)`
  left: `{:#x?}`,
 right: `{:#x?}`: {}"#, &*left_val, &*right_val,
                           format_args!($($arg)+))
                }
            }
        }
    });
}

#[test]
fn no_symbols() -> Result<()> {
    let buf: Vec<u8> = Vec::new();
    let cursor = Cursor::new(buf);
    let builder = Builder::new(
        Header {
            class: Class::ELF32,
            encoding: Encoding::LSB,
            machine: 0x28,     // ARM instruction set
            flags: 0x05000000, // ARM ABI version 5
        },
        cursor,
    )?;
    let cursor = builder.close()?;
    let buf = cursor.into_inner();
    let want: Vec<u8> = vec![
        0x7f, b'E', b'L', b'F', // magic number
        1,    // Class: 32-bit
        1,    // Encoding: little endian
        1,    // Format version
        0,    // No particular ABI
        0, 0, 0, 0, 0, 0, 0, 0, // padding
        0x00, 0x00, // File type: ET_REL
        0x28, 0x00, // Machine type: ARM
        0x01, 0x00, 0x00, 0x00, // Header format version: 1
        0x00, 0x00, 0x00, 0x00, // no entry-point
        0x00, 0x00, 0x00, 0x00, // no program headers
        0x38, 0x00, 0x00, 0x00, // section header offset
        0x00, 0x00, 0x00, 0x05, // Flags: ARM ABI v5
        0x34, 0x00, // Header size: 52
        0x00, 0x00, // Program header entry size (none)
        0x00, 0x00, // Program header entry count (none)
        0x28, 0x00, // Section header entry size
        0x05, 0x00, // Section header entry count
        0x01, 0x00, // Section index of shstrtab
        0, 0, 0, 0, // padding for section header
    ];
    assert_eq_hex!(buf, want);
    Ok(())
}
