use std::fs::File;

use enigo::ParsedKeymap;
use xkbcommon::xkb::{CONTEXT_NO_FLAGS, Context, FORMAT_TEXT_V1, KEYMAP_COMPILE_NO_FLAGS, Keymap};

fn main() {
    let context = Context::new(CONTEXT_NO_FLAGS);
    let format = FORMAT_TEXT_V1;
    let mut keymap_file = File::open("./raw_keymap_file").unwrap();
    let size = 67292;

    // Check if the file size is correct
    let metadata = keymap_file
        .metadata()
        .expect("could not get the file's metadata! Skipping file size check");
    if metadata.len() != size {
        panic!("file does not have the expected size! resetting the keymap");
    }

    let mut parsed_keymap =
        ParsedKeymap::try_from(&mut keymap_file).expect("unable to parse the new keymap");

    let mut keymap_string = format!("{parsed_keymap}");
    while keymap_string.ends_with('\0') {
        println!("removed NULL byte at the end");
        keymap_string.pop();
    }
    let keymap = Keymap::new_from_string(&context, keymap_string, format, KEYMAP_COMPILE_NO_FLAGS)
        .expect("unable to parse the keymap with xkbcommon! resetting the keymap");
    // println!("{}", keymap.get_as_string(format));
    let key = xkeysym::Keysym::from(enigo::Key::Unicode('s'));
    let key_name = key.name().unwrap();
    let key_name = match key_name.strip_prefix("XK_") {
        Some(keyname) => keyname,
        None => key_name,
    };
    parsed_keymap.map_key(key_name).unwrap();
    println!("{parsed_keymap}");

    let mut keymap_string = format!("{parsed_keymap}");
    while keymap_string.ends_with('\0') {
        println!("removed NULL byte at the end");
        keymap_string.pop();
    }

    let keymap = Keymap::new_from_string(&context, keymap_string, format, KEYMAP_COMPILE_NO_FLAGS)
        .expect("unable to parse the keymap with xkbcommon! resetting the keymap");
    println!("{}", keymap.get_as_string(format));
}

/*
Actual code:

 debug!("creating new xkb:Keymap");
        debug!("new(format: {format}, size: {size}, ...)");

        let mut keymap_file = File::from(fd);

        // Check if the file size is correct
        let metadata = keymap_file.metadata().map_err(|e| {
            error!("could not get the file's metadata! Skipping file size check. Error: {e}");
        })?;
        if metadata.len() != size.into() {
            error!("file does not have the expected size! resetting the keymap");
            return Err(());
        }

        let parsed_keymap = ParsedKeymap::try_from(&mut keymap_file).map_err(|_| {
            trace!("unable to parse the new keymap");
        })?;
        // Unfortunately we need to serialize the parsed keymap again, because the
        // xkbcommon parser is super strict and can't handle missing newlines. Ours
        // doesn't mind and when we serialize it, the newlines are added at the correct
        // places so xkbcommon can parse it too
        let mut keymap_string = std::fs::read_to_string("binary_keymap_decoded.txt").unwrap();
        while keymap_string.ends_with('\0') {
            debug!("removed NULL byte at the end");
            keymap_string.pop();
        }
        keymap_string.push('\0');
        debug!("parsed keymap serialized:\n{keymap_string}");
        let keymap =
            Keymap::new_from_string(&context, keymap_string, format, KEYMAP_COMPILE_NO_FLAGS)
                .ok_or({
                    error!("unable to parse the keymap with xkbcommon! resetting the keymap");
                })?;

*/
