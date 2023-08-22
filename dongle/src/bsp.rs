use embassy_nrf::{
    config::HfclkSource,
    gpio::{Input, Level, Output, OutputDrive, Pull},
    peripherals::{P0_00, P0_03},
};
use rtic_monotonics::nrf::timer::Timer0;

pub type DongleLed = Output<'static, P0_00>;
pub type Button = Input<'static, P0_03>;

pub struct DongleBsp {
    pub led: DongleLed,
    pub button: Button,
}

#[inline(always)]
pub fn init_dongle(_: cortex_m::Peripherals) -> DongleBsp {
    defmt::info!("BSP init");

    let mut config = embassy_nrf::config::Config::default();
    config.hfclk_source = HfclkSource::ExternalXtal;
    let p = embassy_nrf::init(config);

    let systick_token = rtic_monotonics::create_nrf_timer0_monotonic_token!();
    Timer0::start(unsafe { core::mem::transmute(()) }, systick_token);

    DongleBsp {
        led: Output::new(p.P0_00, Level::Low, OutputDrive::Standard),
        button: Input::new(p.P0_03, Pull::Up),
    }
}
