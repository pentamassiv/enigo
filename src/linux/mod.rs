cfg_if::cfg_if! {
    if #[cfg(feature = "x11")] {
        mod x11;
        pub use self::x11::Enigo;
    } else {
        mod x11;
        pub use self::x11::Enigo;
        //mod wayland;
        //pub use self::wayland::Enigo;
    }
}

//pub mod keycodes;
