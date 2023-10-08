#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use corne_firmware as _; // global logger + panicking-behavior + memory layout
use rtic_monotonics::{nrf::timer::Timer0, Monotonic};

pub mod keyboard_tasks;

defmt::timestamp!("{=u64:us}", {
    let time_us = Timer0::now().duration_since_epoch().ticks();

    time_us
});

#[rtic::app(device = embassy_nrf::pac, dispatchers = [SWI0_EGU0], peripherals = false)]
mod keyboard_app {
    use crate::keyboard_tasks::*;
    use corne_firmware::{
        bsp::keyboard::{
            init_keyboard, BatteryVoltage, ChargerStatus, KeyMatrix, KeyboardBsp, Led,
        },
        radio::Radio,
    };

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        battery_voltage: BatteryVoltage,
        charger_status: ChargerStatus,
        key_matrix: KeyMatrix,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("pre init");

        let KeyboardBsp {
            radio,
            led,
            battery_voltage,
            charger_status,
            key_matrix,
            is_right_half,
        } = init_keyboard(cx.core);

        key_matrix::spawn().ok();
        battery_handling::spawn().ok();
        radio_task::spawn(radio, is_right_half).ok();

        (
            Shared {},
            Local {
                battery_voltage,
                charger_status,
                key_matrix,
            },
        )
    }

    extern "Rust" {
        #[task(local = [key_matrix])]
        async fn key_matrix(_: key_matrix::Context);

        #[task(local = [battery_voltage, charger_status])]
        async fn battery_handling(_: battery_handling::Context);

        #[task(priority = 3)]
        async fn radio_task(_: radio_task::Context, _: Radio, _: bool);
    }
}
