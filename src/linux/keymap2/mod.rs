use std::{collections::HashSet, fs::File, os::fd::OwnedFd};

use log::{debug, error, trace};
use xkbcommon::xkb::{
    Context, KEYMAP_COMPILE_NO_FLAGS, KeyDirection, Keycode, Keymap, KeymapFormat,
    STATE_LAYOUT_DEPRESSED, STATE_LAYOUT_LATCHED, STATE_LAYOUT_LOCKED, STATE_MODS_DEPRESSED,
    STATE_MODS_LATCHED, STATE_MODS_LOCKED, State,
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
    pressed_keys: HashSet<Keycode>,
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
            pressed_keys: HashSet::with_capacity(8),
        })
    }

    pub fn update(&mut self, format: KeymapFormat, fd: OwnedFd, size: u32) -> Result<(), ()> {
        let depressed_mods = self.state.serialize_mods(STATE_MODS_DEPRESSED);
        let latched_mods = self.state.serialize_mods(STATE_MODS_LATCHED);
        let locked_mods = self.state.serialize_mods(STATE_MODS_LOCKED);

        let depressed_layout = self.state.serialize_layout(STATE_LAYOUT_DEPRESSED);
        let latched_layout = self.state.serialize_layout(STATE_LAYOUT_LATCHED);
        let locked_layout = self.state.serialize_layout(STATE_LAYOUT_LOCKED);

        let Keymap2 {
            context,
            keymap,
            mut state,
            parsed_keymap,
            pressed_keys,
        } = Self::new(self.context.clone(), format, fd, size).map_err(|_| {
            trace!("unable to create new keymap");
        })?;

        // The docs say this is a bad idea. update_key and update_mask should not get
        // mixed. I don't know how else to get the same state though
        for key in pressed_keys {
            state.update_key(key, KeyDirection::Down);
        }

        state.update_mask(
            depressed_mods,
            latched_mods,
            locked_mods,
            depressed_layout,
            latched_layout,
            locked_layout,
        );

        self.context = context;
        self.keymap = keymap;
        self.state = state;
        self.parsed_keymap = parsed_keymap;

        Ok(())
    }

    pub fn update_key(&mut self, keycode: Keycode, direction: KeyDirection) {
        match direction {
            KeyDirection::Up => {
                self.pressed_keys.remove(&keycode);
            }
            KeyDirection::Down => {
                self.pressed_keys.insert(keycode);
            }
        };
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
