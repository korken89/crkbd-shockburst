#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use corne as _; // global logger + panicking-behavior + memory layout
use rtic_monotonics::{nrf::timer::Timer0, Monotonic};

pub mod keyboard_tasks;

defmt::timestamp!("{=u64:us}", {
    let time_us = Timer0::now().duration_since_epoch().ticks();

    time_us
});

#[rtic::app(device = embassy_nrf::pac, dispatchers = [SWI0_EGU0], peripherals = false)]
mod app {
    use crate::keyboard_tasks::*;
    use corne::bsp::{self, BatteryVoltage, Bsp, KeyMatrix};

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        battery_voltage: BatteryVoltage,
        key_matrix: KeyMatrix,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("pre init");

        let Bsp {
            battery_voltage,
            charger_status,
            key_matrix,
        } = bsp::init(cx.core);

        task::spawn().ok();

        (
            Shared {},
            Local {
                battery_voltage,
                key_matrix,
            },
        )
    }

    extern "Rust" {
        #[task(local = [battery_voltage, key_matrix])]
        async fn task(_: task::Context);
    }
}
