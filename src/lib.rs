#![no_std]
#![feature(impl_trait_in_assoc_type)]

pub mod devices;
pub mod events;
pub mod tasks;
pub mod app;
pub mod game_state;

#[macro_export]
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}
