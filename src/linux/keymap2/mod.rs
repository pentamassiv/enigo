use std::io::{self, Read};
use std::os::fd::OwnedFd;

use log::{debug, error, warn};
use xkbcommon::xkb::{
    CONTEXT_NO_FLAGS, Context, KEYMAP_COMPILE_NO_FLAGS, KeyDirection, Keycode, Keymap,
    KeymapFormat, State,
};

use crate::{InputResult, Key, keycodes::ModifierBitflag};

mod parse_keymap;
use parse_keymap::{Parse as _, ParsedKeymap};

#[derive(Clone)]
pub struct Keymap2 {
    context: Context,
    keymap: Keymap,
    state: State,
    parsed_keymap: ParsedKeymap,
}

impl Keymap2 {
    pub fn new(format: KeymapFormat, fd: OwnedFd, size: u32) -> Result<Self, ()> {
        use std::io::{Read, Seek, SeekFrom};

        debug!("creating new xkb:Keymap");

        // Read keymap to String
        let mut keymap_file = std::fs::File::from(fd);
        let mut keymap_str = String::new();
        keymap_file.read_to_string(&mut keymap_str).map_err(|e| {
            error!("unable to read file to string:\n{e}");
        })?;

        // Reset the cursor to the beginning of the file.
        keymap_file.seek(SeekFrom::Start(0)).map_err(|e| {
            error!("unable to seek from the start:\n{e}");
        })?;

        // Parse the keymap
        let (remaining, parsed_keymap) = ParsedKeymap::parse(&keymap_str).unwrap();
        if remaining != "" {
            warn!("not all of the keymap could be parsed")
        }

        let context = Context::new(CONTEXT_NO_FLAGS);
        let keymap = Self::new_keymap(&context, format, &mut keymap_file, size)?;
        let state = State::new(&keymap);

        Ok(Self {
            context,
            keymap,
            state,
            parsed_keymap,
        })
    }

    pub fn update(&mut self, format: KeymapFormat, fd: OwnedFd, size: u32) -> Result<(), ()> {
        let xkb_keymap = Self::new_keymap(&self.context, format, fd, size);
        self.keymap = xkb_keymap;
        todo!()
    }

    pub fn update_state(&mut self, keycode: Keycode, direction: KeyDirection) {
        self.state.update_key(keycode, direction);
    }

    pub fn update_modifiers(&mut self, new_modifier_bitflag: u32) {
        debug!("updating xkb:Keymap");
        todo!()
    }

    pub fn get_file(&self) -> (u32, ::std::os::unix::io::BorrowedFd<'_>, u32) {
        let keymap_serialized = format!("{keymap}");
        debug!("updating xkb:Keymap");
        todo!()
    }

    pub fn is_modifier(&self, keycode: u16) -> Option<ModifierBitflag> {
        debug!("updating xkb:Keymap");
        todo!()
    }

    pub fn key_to_keycode(&self, key: Key) -> Option<u16> {
        debug!("updating xkb:Keymap");
        todo!()
    }

    pub fn map_key(&self, key: Key) -> InputResult<u16> {
        debug!("updating xkb:Keymap");
        todo!()
    }

    fn new_keymap(
        context: &Context,
        format: KeymapFormat,
        keymap_file: &mut std::fs::File,
        size: u32,
    ) -> Result<Keymap, ()> {
        // Check if the file size is correct
        let Ok(metadata) = keymap_file.metadata() else {
            error!(
                "could not get the files metadata! skipping file size check and resetting the keymap"
            );
            return Err(());
        };
        if metadata.len() != size.into() {
            error!("file does not have the expected size! resetting the keymap");
            return Err(());
        }

        let flags = KEYMAP_COMPILE_NO_FLAGS;

        // Try creating keymap
        let Some(xkb_keymap) = Keymap::new_from_file(context, keymap_file, format, flags) else {
            error!("Creating xkb:Keymap failed! resetting the keymap");
            return Err(());
        };
        Ok(xkb_keymap)
    }
}

impl Default for Keymap2 {
    fn default() -> Self {
        todo!()
    }
}
