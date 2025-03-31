use std::{fs::File, os::fd::OwnedFd};

use log::{debug, error, trace};
use xkbcommon::xkb::{
    Context, KEYMAP_COMPILE_NO_FLAGS, KeyDirection, Keycode, Keymap, KeymapFormat, State,
};

use crate::{InputResult, Key, keycodes::ModifierBitflag};

mod parse_keymap;
use parse_keymap::ParsedKeymap;

#[derive(Clone)]
pub struct Keymap2 {
    context: Context,
    keymap: Keymap,
    state: State,
    parsed_keymap: ParsedKeymap,
}

impl Keymap2 {
    pub fn new(context: Context, format: KeymapFormat, fd: OwnedFd, size: u32) -> Result<Self, ()> {
        debug!("creating new xkb:Keymap");

        let mut keymap_file = File::from(fd);
        let parsed_keymap = ParsedKeymap::try_from(&mut keymap_file).map_err(|_| {
            trace!("unable to parse the new keymap");
        })?;
        let keymap = Self::new_xkb_keymap(&context, format, &mut keymap_file, size)?;
        let state = State::new(&keymap);

        Ok(Self {
            context,
            keymap,
            state,
            parsed_keymap,
        })
    }

    pub fn update(&mut self, format: KeymapFormat, fd: OwnedFd, size: u32) -> Result<(), ()> {
        let new_keymap = Self::new(self.context.clone(), format, fd, size).map_err(|_| {
            trace!("unable to create new keymap");
        })?;

        // TODO: update the state here

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

    fn new_xkb_keymap(
        context: &Context,
        format: KeymapFormat,
        keymap_file: &mut File,
        size: u32,
    ) -> Result<Keymap, ()> {
        // Check if the file size is correct.
        let metadata = keymap_file.metadata().map_err(|e| {
            error!("could not get the file's metadata! Skipping file size check. Error: {e}");
        })?;
        if metadata.len() != size.into() {
            error!("file does not have the expected size! resetting the keymap");
            return Err(());
        }

        let flags = KEYMAP_COMPILE_NO_FLAGS;

        // Try creating keymap.
        Keymap::new_from_file(context, keymap_file, format, flags).ok_or_else(|| {
            error!("Creating xkb:Keymap failed! resetting the keymap");
        })
    }
}

impl Default for Keymap2 {
    fn default() -> Self {
        todo!()
    }
}
