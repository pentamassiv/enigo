use std::{
    collections::VecDeque,
    convert::TryInto as _,
    env,
    num::Wrapping,
    os::unix::{io::AsFd, net::UnixStream},
    path::PathBuf,
    time::Instant,
};

use log::{debug, error, trace, warn};
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle,
    protocol::{
        wl_keyboard::{self, WlKeyboard},
        wl_pointer::{self, WlPointer},
        wl_registry,
        wl_seat::{self, Capability},
    },
};
use wayland_protocols_misc::{
    zwp_input_method_v2::client::{zwp_input_method_manager_v2, zwp_input_method_v2},
    zwp_virtual_keyboard_v1::client::{zwp_virtual_keyboard_manager_v1, zwp_virtual_keyboard_v1},
};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1, zwlr_virtual_pointer_v1,
};

use super::keymap::{Bind, KeyMap};
use crate::{
    Axis, Button, Coordinate, Direction, InputError, InputResult, Key, Keyboard, Mouse,
    NewConError, keycodes::Modifier, keycodes::ModifierBitflag,
};

pub type Keycode = u32;

pub struct Con {
    keymap: KeyMap<Keycode>,
    event_queue: EventQueue<WaylandState>,
    state: WaylandState,
    virtual_keyboard: Option<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1>,
    input_method: Option<zwp_input_method_v2::ZwpInputMethodV2>,
    virtual_pointer: Option<zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1>,
    base_time: std::time::Instant,
}

impl Con {
    /// Tries to establish a new Wayland connection
    ///
    /// # Errors
    /// TODO
    pub fn new(dpy_name: Option<&str>) -> Result<Self, NewConError> {
        // Setup Wayland connection
        let connection = Self::setup_connection(dpy_name)?;

        // Check to see if there was an error trying to connect
        if let Some(e) = connection.protocol_error() {
            error!(
                "unknown wayland initialization failure: {} {} {} {}",
                e.code, e.object_id, e.object_interface, e.message
            );
            return Err(NewConError::EstablishCon(
                "failed to connect to wayland. there was a protocol error",
            ));
        }

        // Create the event queue
        let mut event_queue = connection.new_event_queue();
        // Get queue handle
        let qh = event_queue.handle();

        // Start registry
        let display = connection.display();
        let registry = display.get_registry(&qh, ());

        // Setup WaylandState and store the globals in it
        let mut state = WaylandState::default();
        event_queue
            .roundtrip(&mut state)
            .map_err(|_| NewConError::EstablishCon("Wayland roundtrip failed"))?;

        let keymap = KeyMap::new(
            8,
            255,
            // All keycodes are unused when initialized
            (8..=255).collect::<VecDeque<Keycode>>(),
            0,
            Vec::new(),
        );

        let mut connection = Self {
            keymap,
            event_queue,
            state,
            virtual_keyboard: None,
            input_method: None,
            virtual_pointer: None,
            base_time: Instant::now(),
        };

        connection.bind_globals(&registry)?;

        connection.init_protocols()?;

        connection
            .apply_keymap()
            .map_err(|_| NewConError::EstablishCon("Unable to apply the keymap"))?;

        Ok(connection)
    }

    // Helper function for setting up the Wayland connection
    fn setup_connection(dyp_name: Option<&str>) -> Result<Connection, NewConError> {
        let connection = if let Some(dyp_name) = dyp_name {
            debug!(
                "\x1b[93mtrying to establish a connection to: {}\x1b[0m",
                dyp_name
            );
            let socket_path = env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).ok_or(
                NewConError::EstablishCon("Missing XDG_RUNTIME_DIR env variable"),
            )?;
            let stream = UnixStream::connect(socket_path.join(dyp_name))
                .map_err(|_| NewConError::EstablishCon("Failed to open Unix stream"))?;
            Connection::from_socket(stream)
        } else {
            debug!("\x1b[93mtrying to establish a connection to $WAYLAND_DISPLAY\x1b[0m");
            Connection::connect_to_env()
        };

        connection.map_err(|_| {
            error!("Failed to connect to Wayland. Try setting 'WAYLAND_DISPLAY=wayland-0'.");
            NewConError::EstablishCon("Wayland connection failed.")
        })
    }

    fn bind_globals(&mut self, registry: &wl_registry::WlRegistry) -> Result<(), NewConError> {
        let qh = self.event_queue.handle();

        // Bind to wl_seat if it exists
        // MUST be done before doing any bindings relevant to the input_method
        // protocol, otherwise e.g. labwc crashes
        let &(name, version) = self
            .state
            .globals
            .get("wl_seat")
            .ok_or(NewConError::EstablishCon("No seat available"))?;
        let seat = registry.bind::<wl_seat::WlSeat, _, _>(name, version.min(1), &qh, ());

        self.event_queue
            .flush()
            .map_err(|_| NewConError::EstablishCon("Flushing Wayland queue failed"))?;
        self.state.seat = Some(seat);

        // Wait for compositor to handle the request and send back the capabilities of
        // the seat
        // The WlPointer and/or WlKeyboard get created now if the seat has the
        // capabilities for it
        debug!("waiting for response of request to bind to seat");
        self.event_queue
            .blocking_dispatch(&mut self.state)
            .map_err(|_| NewConError::EstablishCon("Wayland blocking dispatch failed"))?;

        // Send the events to the compositor to handle them
        self.event_queue
            .flush()
            .map_err(|_| NewConError::EstablishCon("Flushing Wayland queue failed"))?;

        // Wait for compositor to create the WlPointer and WlKeyboard and get the keymap
        // of the WlKeyboard
        debug!("asked to create keyboard and pointer");
        self.event_queue
            .blocking_dispatch(&mut self.state)
            .map_err(|_| NewConError::EstablishCon("Wayland blocking dispatch failed"))?;

        // Ask compositor to create VirtualKeyboardManager
        if let Some(&(name, version)) = self.state.globals.get("zwp_virtual_keyboard_manager_v1") {
            let manager = registry
                .bind::<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1, _, _>(
                    name,
                    version.min(1),
                    &qh,
                    (),
                );
            self.event_queue
                .flush()
                .map_err(|_| NewConError::EstablishCon("Flushing Wayland queue failed"))?;
            self.state.keyboard_manager = Some(manager);
        }

        // Ask compositor to create InputMethodManager
        if let Some(&(name, version)) = self.state.globals.get("zwp_input_method_manager_v2") {
            let manager = registry
                .bind::<zwp_input_method_manager_v2::ZwpInputMethodManagerV2, _, _>(
                    name,
                    version.min(1),
                    &qh,
                    (),
                );
            self.event_queue
                .flush()
                .map_err(|_| NewConError::EstablishCon("Flushing Wayland queue failed"))?;
            self.state.im_manager = Some(manager);
        }

        // Ask compositor to create VirtualPointerManager
        if let Some(&(name, version)) = self.state.globals.get("zwlr_virtual_pointer_manager_v1") {
            let manager = registry
                .bind::<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1, _, _>(
                    name,
                    version.min(1),
                    &qh,
                    (),
                );
            self.event_queue
                .flush()
                .map_err(|_| NewConError::EstablishCon("Flushing Wayland queue failed"))?;
            self.state.pointer_manager = Some(manager);
        }

        Ok(())
    }

    /// Try to set up all the protocols. An error is returned, if no protocol is
    /// available
    fn init_protocols(&mut self) -> Result<(), NewConError> {
        let qh = self.event_queue.handle();

        if self.state.seat.is_some() {
            // Setup input method
            self.input_method =
                self.state.im_manager.as_ref().map(|im_mgr| {
                    im_mgr.get_input_method(self.state.seat.as_ref().unwrap(), &qh, ())
                });
            // Wait for Activate response if the input_method was created
            if self.input_method.is_some() {
                self.event_queue
                    .blocking_dispatch(&mut self.state)
                    .map_err(|_| NewConError::EstablishCon("Wayland blocking dispatch failed"))?;
            }

            // Setup virtual keyboard
            self.virtual_keyboard = self.state.keyboard_manager.as_ref().map(|vk_mgr| {
                vk_mgr.create_virtual_keyboard(self.state.seat.as_ref().unwrap(), &qh, ())
            });
            // Wait for KeyMap response if virtual_keyboard was created
            if self.virtual_keyboard.is_some() {
                self.event_queue
                    .blocking_dispatch(&mut self.state)
                    .map_err(|_| NewConError::EstablishCon("Wayland blocking dispatch failed"))?;
            }
        }

        // Setup virtual pointer
        self.virtual_pointer = self
            .state
            .pointer_manager
            .as_ref()
            .map(|vp_mgr| vp_mgr.create_virtual_pointer(self.state.seat.as_ref(), &qh, ()));
        if self.virtual_pointer.is_some() {
            self.event_queue
                .flush()
                .map_err(|_| NewConError::EstablishCon("Flushing Wayland queue failed"))?;
        }

        debug!("create virtual keyboard is done");

        debug!(
            "protocols available\nvirtual_keyboard: {}\ninput_method: {}\nvirtual_pointer: {}",
            self.virtual_keyboard.is_some(),
            self.input_method.is_some(),
            self.virtual_pointer.is_some(),
        );

        if self.virtual_keyboard.is_none()
            && self.input_method.is_none()
            && self.virtual_pointer.is_none()
        {
            return Err(NewConError::EstablishCon(
                "no protocol available to simulate input",
            ));
        }
        Ok(())
    }

    /// Get the duration since the Keymap was created
    fn get_time(&self) -> u32 {
        let duration = self.base_time.elapsed();
        let time = duration.as_millis();
        time.try_into().unwrap_or(u32::MAX)
    }

    /// Press/Release a keycode
    ///
    /// # Errors
    /// TODO
    fn send_key_event(&mut self, keycode: Keycode, direction: Direction) -> InputResult<()> {
        let vk = self
            .virtual_keyboard
            .as_ref()
            .ok_or(InputError::Simulate("no way to enter key"))?;
        is_alive(vk)?;
        let time = self.get_time();
        let keycode = keycode - 8; // Adjust by 8 due to the xkb/xwayland requirements

        if direction == Direction::Press || direction == Direction::Click {
            trace!("vk.key({time}, {keycode}, 1)");
            vk.key(time, keycode, 1);
            self.event_queue
                .flush()
                .map_err(|_| InputError::Simulate("Flushing Wayland queue failed"))?;
        }
        if direction == Direction::Release || direction == Direction::Click {
            trace!("vk.key({time}, {keycode}, 0)");
            vk.key(time, keycode, 0);
            self.event_queue
                .flush()
                .map_err(|_| InputError::Simulate("Flushing Wayland queue failed"))?;
        }
        Ok(())
    }

    /// Sends a modifier event with the updated bitflag of the modifiers to the
    /// compositor
    fn send_modifier_event(&mut self, modifiers: ModifierBitflag) -> InputResult<()> {
        // Retrieve virtual keyboard or return an error early if None
        let vk = self
            .virtual_keyboard
            .as_ref()
            .ok_or(InputError::Simulate("no way to enter key"))?;

        // Check if virtual keyboard is still alive
        is_alive(vk)?;

        // Log the modifier event
        trace!("vk.modifiers({modifiers}, 0, 0, 0)");

        // Send the modifier event
        vk.modifiers(modifiers, 0, 0, 0);

        self.event_queue
            .flush()
            .map_err(|_| InputError::Simulate("Flushing Wayland queue failed"))?;

        Ok(())
    }

    /// Apply the current keymap
    ///
    /// # Errors
    /// TODO
    fn apply_keymap(&mut self) -> InputResult<()> {
        trace!("apply_keymap(&mut self)");
        let vk = self
            .virtual_keyboard
            .as_ref()
            .ok_or(InputError::Simulate("no way to apply keymap"))?;
        is_alive(vk)?;

        // Regenerate keymap and handle failure
        let keymap_size = self
            .keymap
            .regenerate()
            .map_err(|_| InputError::Mapping("unable to regenerate keymap".to_string()))?;

        // Early return if the keymap was not changed because we only send an updated
        // keymap if we had to regenerate it
        let Some(size) = keymap_size else {
            return Ok(());
        };

        trace!("update wayland keymap");

        let keymap_file = self.keymap.file.as_ref().unwrap(); // Safe here, assuming file is always present
        vk.keymap(1, keymap_file.as_fd(), size);

        debug!("wait for response after keymap call");
        self.event_queue
            .blocking_dispatch(&mut self.state)
            .map_err(|_| InputError::Simulate("Wayland blocking_dispatch failed"))?;

        Ok(())
    }

    fn raw(&mut self, keycode: Keycode, direction: Direction) -> InputResult<()> {
        // Apply the new keymap if there were any changes
        self.apply_keymap()?;

        // Send the key event and update keymap state
        // This is important to avoid unmapping held keys
        self.send_key_event(keycode, direction)?;
        self.keymap.key(keycode, direction);

        Ok(())
    }

    /// Flush the Wayland queue
    fn flush(&self) -> InputResult<()> {
        self.event_queue.flush().map_err(|e| {
            error!("{:?}", e);
            InputError::Simulate("could not flush Wayland queue")
        })?;
        trace!("flushed event queue");
        Ok(())
    }
}

impl Bind<Keycode> for Con {
    // Nothing to do
    // On Wayland only the whole keymap can be applied
}

impl Drop for Con {
    // Destroy the Wayland objects we created
    fn drop(&mut self) {
        if let Some(vk) = self.virtual_keyboard.take() {
            vk.destroy();
        }
        if let Some(im) = self.input_method.take() {
            im.destroy();
        }
        if let Some(vp) = self.virtual_pointer.take() {
            vp.destroy();
        }

        if self.flush().is_err() {
            error!("could not flush wayland queue");
        }
        trace!("wayland objects were destroyed");

        let _ = self.event_queue.roundtrip(&mut self.state);
    }
}

#[derive(Clone, Debug, Default)]
/// Stores the manager for the various protocols
struct WaylandState {
    // Map of interface name -> (global name, version)
    globals: std::collections::HashMap<String, (u32, u32)>,
    keyboard_manager: Option<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1>,
    im_manager: Option<zwp_input_method_manager_v2::ZwpInputMethodManagerV2>,
    im_serial: Wrapping<u32>,
    pointer_manager: Option<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1>,
    seat: Option<wl_seat::WlSeat>,
    seat_keyboard: Option<WlKeyboard>,
    seat_pointer: Option<WlPointer>,
    /*  output: Option<wl_output::WlOutput>,
    width: i32,
    height: i32,*/
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // When receiving events from the wl_registry, we are only interested in the
        // `global` event, which signals a new available global and then store it to
        // later bind to them
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            trace!(
                "Global announced: {} (name: {}, version: {})",
                interface, name, version
            );
            state.globals.insert(interface, (name, version));
        }
    }
}

impl Dispatch<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _manager: &zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
        event: zwp_virtual_keyboard_manager_v1::Event,
        (): &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        warn!("Received a virtual keyboard manager event {:?}", event);
    }
}

impl Dispatch<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _vk: &zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
        event: zwp_virtual_keyboard_v1::Event,
        (): &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        warn!("Got a virtual keyboard event {:?}", event);
    }
}

impl Dispatch<zwp_input_method_manager_v2::ZwpInputMethodManagerV2, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _manager: &zwp_input_method_manager_v2::ZwpInputMethodManagerV2,
        event: zwp_input_method_manager_v2::Event,
        (): &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        warn!("Received an input method manager event {:?}", event);
    }
}
impl Dispatch<zwp_input_method_v2::ZwpInputMethodV2, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _vk: &zwp_input_method_v2::ZwpInputMethodV2,
        event: zwp_input_method_v2::Event,
        (): &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        warn!("Got a input method event {:?}", event);
        match event {
            zwp_input_method_v2::Event::Done => state.im_serial += Wrapping(1u32),
            _ => (), // TODO
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WaylandState {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        (): &(),
        _con: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        warn!("Received a seat event {:?}", event);
        if let wl_seat::Event::Capabilities { capabilities } = event {
            let capabilities = match capabilities {
                wayland_client::WEnum::Value(capabilities) => capabilities,
                wayland_client::WEnum::Unknown(v) => {
                    warn!("Unknown value for the capabilities of the wl_seat: {v}");
                    return;
                }
            };

            // Create a WlKeyboard if the seat has the capability
            if state.seat_keyboard.is_none() && capabilities.contains(Capability::Keyboard) {
                let seat_keyboard = seat.get_keyboard(qh, ());
                state.seat_keyboard = Some(seat_keyboard);
            }

            // Create a WlPointer if the seat has the capability
            if state.seat_pointer.is_none() && capabilities.contains(Capability::Pointer) {
                let seat_pointer = seat.get_pointer(qh, ());
                state.seat_pointer = Some(seat_pointer);
            }
        } else {
            // TODO: Handle the case of removed capabilities
            warn!("Event was not handled");
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _seat: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        (): &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        warn!("Got a wl_keyboard event {:?}", event);
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _seat: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        (): &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        warn!("Got a wl_pointer event {:?}", event);
    }
}

/*
impl Dispatch<wl_output::WlOutput, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _output: &wl_output::WlOutput,
        event: wl_output::Event,
        (): &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_output::Event::Geometry {
                x,
                y,
                physical_width,
                physical_height,
                subpixel,
                make,
                model,
                transform,
            } => {
                state.width = x;
                state.height = y;
                warn!("x: {}, y: {}, physical_width: {}, physical_height: {}, make: {}, : {}",x,y,physical_width,physical_height,make,model,model);
            }
            wl_output::Event::Mode {
                flags,
                width,
                height,
                refresh,
            } => {
                warn!("width: {}, : {height}",width,height);
            }
            _ => {}
        };
    }
}*/

impl Dispatch<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _manager: &zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
        event: zwlr_virtual_pointer_manager_v1::Event,
        (): &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        warn!("Received a virtual keyboard manager event {:?}", event);
    }
}

impl Dispatch<zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _vk: &zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
        event: zwlr_virtual_pointer_v1::Event,
        (): &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        warn!("Got a virtual keyboard event {:?}", event);
    }
}

impl Drop for WaylandState {
    // Destroy the manager for the protocols we used
    fn drop(&mut self) {
        if let Some(im_mgr) = self.im_manager.as_ref() {
            im_mgr.destroy();
        }
        if let Some(pointer_mgr) = self.pointer_manager.as_ref() {
            pointer_mgr.destroy();
        }
    }
}

impl Keyboard for Con {
    fn fast_text(&mut self, text: &str) -> InputResult<Option<()>> {
        let Some(im) = self.input_method.as_mut() else {
            return Ok(None);
        };

        is_alive(im)?;
        trace!("fast text input with imput_method protocol");
        // Process all previous events so that the serial number is correct
        self.event_queue
            .roundtrip(&mut self.state)
            .map_err(|_| InputError::Simulate("The roundtrip on Wayland failed"))?;
        im.commit_string(text.to_string());
        im.commit(self.state.im_serial.0);

        self.event_queue
            .flush()
            .map_err(|_| InputError::Simulate("Flushing Wayland queue failed"))?;

        Ok(Some(()))
    }

    fn key(&mut self, key: Key, direction: Direction) -> InputResult<()> {
        let Ok(modifier) = Modifier::try_from(key) else {
            let keycode = self.keymap.key_to_keycode(&(), key)?;
            self.raw(keycode, direction)?;
            return Ok(());
        };

        // Send the events to the compositor
        trace!("it is a modifier: {modifier:?}");
        if direction == Direction::Click || direction == Direction::Press {
            let modifiers = self
                .keymap
                .enter_modifier(modifier.bitflag(), Direction::Press);
            self.send_modifier_event(modifiers)?;
        }
        if direction == Direction::Click || direction == Direction::Release {
            let modifiers = self
                .keymap
                .enter_modifier(modifier.bitflag(), Direction::Release);
            self.send_modifier_event(modifiers)?;
        }

        Ok(())
    }

    fn raw(&mut self, keycode: u16, direction: Direction) -> InputResult<()> {
        self.raw(keycode as u32, direction)
    }
}
impl Mouse for Con {
    fn button(&mut self, button: Button, direction: Direction) -> InputResult<()> {
        let vp = self
            .virtual_pointer
            .as_ref()
            .ok_or(InputError::Simulate("no way to enter button"))?;

        // Do nothing if one of the mouse scroll buttons was released
        // Releasing one of the scroll mouse buttons has no effect
        if direction == Direction::Release
            && matches!(
                button,
                Button::ScrollDown | Button::ScrollUp | Button::ScrollRight | Button::ScrollLeft
            )
        {
            return Ok(());
        }

        let button = match button {
            // Taken from /linux/input-event-codes.h
            Button::Left => 0x110,
            Button::Right => 0x111,
            Button::Back => 0x116,
            Button::Forward => 0x115,
            Button::Middle => 0x112,
            Button::ScrollDown => return self.scroll(1, Axis::Vertical),
            Button::ScrollUp => return self.scroll(-1, Axis::Vertical),
            Button::ScrollRight => return self.scroll(1, Axis::Horizontal),
            Button::ScrollLeft => return self.scroll(-1, Axis::Horizontal),
        };

        if direction == Direction::Press || direction == Direction::Click {
            let time = self.get_time();
            trace!("vp.button({time}, {button}, wl_pointer::ButtonState::Pressed)");
            vp.button(time, button, wl_pointer::ButtonState::Pressed);
            vp.frame(); // TODO: Check if this is needed
        }

        if direction == Direction::Release || direction == Direction::Click {
            let time = self.get_time();
            trace!("vp.button({time}, {button}, wl_pointer::ButtonState::Released)");
            vp.button(time, button, wl_pointer::ButtonState::Released);
            vp.frame(); // TODO: Check if this is needed
        }
        self.event_queue
            .flush()
            .map_err(|_| InputError::Simulate("Flushing Wayland queue failed"))
    }

    fn move_mouse(&mut self, x: i32, y: i32, coordinate: Coordinate) -> InputResult<()> {
        let vp = self
            .virtual_pointer
            .as_ref()
            .ok_or(InputError::Simulate("no way to move the mouse"))?;

        let time = self.get_time();
        match coordinate {
            Coordinate::Rel => {
                trace!("vp.motion({time}, {x}, {y})");
                vp.motion(time, x as f64, y as f64);
            }
            Coordinate::Abs => {
                let x: u32 = x.try_into().map_err(|_| {
                    InputError::InvalidInput("the absolute coordinates cannot be negative")
                })?;
                let y: u32 = y.try_into().map_err(|_| {
                    InputError::InvalidInput("the absolute coordinates cannot be negative")
                })?;

                trace!("vp.motion_absolute({time}, {x}, {y}, u32::MAX, u32::MAX)");
                vp.motion_absolute(
                    time,
                    x,
                    y,
                    u32::MAX, // TODO: Check what would be the correct value here
                    u32::MAX, // TODO: Check what would be the correct value here
                );
            }
        }
        vp.frame(); // TODO: Check if this is needed

        // TODO: Change to flush()
        self.event_queue
            .roundtrip(&mut self.state)
            .map_err(|_| InputError::Simulate("The roundtrip on Wayland failed"))
            .map(|_| ())
    }

    fn scroll(&mut self, length: i32, axis: Axis) -> InputResult<()> {
        let vp = self
            .virtual_pointer
            .as_ref()
            .ok_or(InputError::Simulate("no way to scroll"))?;

        // TODO: Check what the value of length should be
        // TODO: Check if it would be better to use .axis_discrete here
        let time = self.get_time();
        let axis = match axis {
            Axis::Horizontal => wl_pointer::Axis::HorizontalScroll,
            Axis::Vertical => wl_pointer::Axis::VerticalScroll,
        };
        trace!("vp.axis(time, axis, length.into())");
        vp.axis(time, axis, length.into());
        vp.frame(); // TODO: Check if this is needed

        // TODO: Change to flush()
        self.event_queue
            .roundtrip(&mut self.state)
            .map_err(|_| InputError::Simulate("The roundtrip on Wayland failed"))
            .map(|_| ())
    }

    fn main_display(&self) -> InputResult<(i32, i32)> {
        // TODO Implement this
        error!(
            "You tried to get the dimensions of the main display. I don't know how this is possible under Wayland. Let me know if there is a new protocol"
        );
        Err(InputError::Simulate("Not implemented yet"))
    }

    fn location(&self) -> InputResult<(i32, i32)> {
        // TODO Implement this
        error!(
            "You tried to get the mouse location. I don't know how this is possible under Wayland. Let me know if there is a new protocol"
        );
        Err(InputError::Simulate("Not implemented yet"))
    }
}

fn is_alive<P: wayland_client::Proxy>(proxy: &P) -> InputResult<()> {
    if proxy.is_alive() {
        Ok(())
    } else {
        Err(InputError::Simulate("wayland proxy is dead"))
    }
}
