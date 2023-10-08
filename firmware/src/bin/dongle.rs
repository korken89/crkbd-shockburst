#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use corne_firmware as _; // global logger + panicking-behavior + memory layout
use rtic_monotonics::{nrf::timer::Timer0, Monotonic};

pub mod dongle_tasks;

defmt::timestamp!("{=u64:us}", {
    let time_us = Timer0::now().duration_since_epoch().ticks();

    time_us
});

#[rtic::app(device = embassy_nrf::pac, dispatchers = [SWI0_EGU0], peripherals = false)]
mod dongle_app {
    use crate::dongle_tasks::*;
    use corne_firmware::{
        bsp::{dongle::init_dongle, dongle::DongleBsp},
        radio::Radio,
    };

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("pre init");

        let DongleBsp { led, button, radio } = init_dongle(cx.core);

        radio_task::spawn(radio).ok();
        // usb_task::spawn().ok();

        (Shared {}, Local {})
    }

    extern "Rust" {
        #[task(priority = 3)]
        async fn radio_task(_: radio_task::Context, _: Radio);
    }
}
