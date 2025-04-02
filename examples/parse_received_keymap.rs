use std::{
    fs::File,
    io::{Read as _, Write},
};

use xkbcommon::xkb::{CONTEXT_NO_FLAGS, Context, FORMAT_TEXT_V1, KEYMAP_COMPILE_NO_FLAGS, Keymap};

fn main() {
    /*
    DIFF:
    $ xxd binary_keymap.txt | head -n 1
    00000000: 3132 302c 2031 3037 2c20 3938 2c20 3935  120, 107, 98, 95
    $ xxd binary_keymap_decoded.txt | head -n 1
    00000000: 786b 625f 6b65 796d 6170 207b 0a78 6b62  xkb_keymap {.xkb
    */

    /*
    let context = Context::new(CONTEXT_NO_FLAGS);
    let format = FORMAT_TEXT_V1;

    let mut keymap_file = File::open("./binary_keymap.txt").unwrap();
    let mut file_content = String::new();
    let keymap = keymap_file
        .read_to_string(&mut file_content)
        .ok()
        .and_then(|_| {
            Keymap::new_from_string(&context, file_content, format, KEYMAP_COMPILE_NO_FLAGS)
        })
        .or_else(|| {
            Keymap::new_from_file(&context, &mut keymap_file, format, KEYMAP_COMPILE_NO_FLAGS)
        })
        .ok_or_else(|| println!("Creating xkb::Keymap failed! resetting the keymap"))
        .unwrap();

    println!("{}", keymap.get_as_string(format));
    // */

    /*
    let content = std::fs::read_to_string("binary_keymap.txt").unwrap();

    // Split the content by commas, trim whitespace, parse to u8, and convert to a
    // char.
    let decoded: String = content
        .split(',')
        .filter_map(|s| s.trim().parse::<u8>().ok().map(char::from))
        .collect();

    let mut write_file = std::fs::File::create("./binary_keymap_decoded.txt").unwrap();
    write!(write_file, "{}", decoded).unwrap();
    // println!("{}", decoded);
    // */

    let received_keymap = std::fs::read_to_string("binary_keymap_decoded.txt").unwrap();
    println!("{}", received_keymap);
    let context = Context::new(CONTEXT_NO_FLAGS);
    let format = FORMAT_TEXT_V1;

    let keymap =
        Keymap::new_from_string(&context, received_keymap, format, KEYMAP_COMPILE_NO_FLAGS);
    if let Some(keymap) = keymap {
        println!("{}", keymap.get_as_string(format));
        return;
    }

    /*
    let mut keymap_file = File::open("./binary_keymap_decoded.txt").unwrap();
    let context = Context::new(CONTEXT_NO_FLAGS);
    let format = FORMAT_TEXT_V1;
    let keymap =
        Keymap::new_from_file(&context, &mut keymap_file, format, KEYMAP_COMPILE_NO_FLAGS).unwrap();
    println!("{}", keymap.get_as_string(format));*/
}
