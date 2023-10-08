use super::start_timer0_monotonic;
use crate::radio::Radio;
use embassy_nrf::{
    config::HfclkSource,
    gpio::{Input, Level, Output, OutputDrive, Pull},
    pac,
    peripherals::{P0_00, P0_03},
};

pub use super::Mono;

pub type DongleLed = Output<'static, P0_00>;
pub type Button = Input<'static, P0_03>;

pub struct DongleBsp {
    pub led: DongleLed,
    pub button: Button,
    pub radio: Radio,
}

#[inline(always)]
pub fn init_dongle(_: cortex_m::Peripherals) -> DongleBsp {
    defmt::info!("Dongle BSP init");

    let mut config = embassy_nrf::config::Config::default();
    config.hfclk_source = HfclkSource::ExternalXtal;
    let p = embassy_nrf::init(config);

    start_timer0_monotonic(p.PPI_CH0);

    // SAFETY: Embassy does not support radio, so we conjure it from the PAC.
    let radio: pac::RADIO = unsafe { core::mem::transmute(()) };
    let radio = Radio::init(radio);

    DongleBsp {
        led: Output::new(p.P0_00, Level::Low, OutputDrive::Standard),
        button: Input::new(p.P0_03, Pull::Up),
        radio,
    }
}
