#![no_std]
#![feature(type_alias_impl_trait)]
#![allow(incomplete_features)]

#[cfg(test)]
#[macro_use]
extern crate std;

use defmt_rtt as _; // global logger
                    // pub use nrf52840_hal as hal; // memory layout

use panic_probe as _;

pub mod bsp;
pub mod radio;
pub mod radio_protocol;
pub mod waker_registration;

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

/// Terminates the application and makes `probe-run` exit with exit-code = 0
pub fn exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}
