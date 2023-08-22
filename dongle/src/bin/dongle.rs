#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use corne as _; // global logger + panicking-behavior + memory layout
use rtic_monotonics::{nrf::timer::Timer0, Monotonic};

pub mod dongle_tasks;

defmt::timestamp!("{=u64:us}", {
    let time_us = Timer0::now().duration_since_epoch().ticks();

    time_us
});

#[rtic::app(device = embassy_nrf::pac, dispatchers = [SWI0_EGU0], peripherals = false)]
mod app {
    use crate::dongle_tasks::*;
    use corne::bsp::{self, DongleBsp, DongleLed};

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: DongleLed,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("pre init");

        let DongleBsp { led, button } = bsp::init_dongle(cx.core);

        task::spawn().ok();

        (Shared {}, Local { led })
    }

    extern "Rust" {
        #[task(local = [led])]
        async fn task(_: task::Context);
    }
}
