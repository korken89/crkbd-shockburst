use embassy_nrf::{
    config::HfclkSource,
    gpio::{AnyPin, Input, Level, Output, OutputDrive, Pin, Pull},
    peripherals::P0_20,
    saadc::Saadc,
    {bind_interrupts, saadc},
};
use keyberon::matrix::Matrix;
use rtic_monotonics::nrf::timer::Timer0;

pub struct ChargerStatus {
    stat: Input<'static, P0_20>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, defmt::Format, Hash)]
pub enum ChargingStatus {
    /// The battery is charging.
    Charging,
    // The charging has finished.
    ChargeComplete,
}

impl ChargerStatus {
    pub fn status(&self) -> ChargingStatus {
        let stat_low = self.stat.is_low();

        match stat_low {
            true => ChargingStatus::Charging,
            false => ChargingStatus::ChargeComplete,
        }
    }
}

bind_interrupts!(struct Irqs {
    SAADC => saadc::InterruptHandler;
});

pub type KeyMatrix = Matrix<Input<'static, AnyPin>, Output<'static, AnyPin>, 6, 4>;

pub struct Bsp {
    pub battery_voltage: BatteryVoltage,
    pub charger_status: ChargerStatus,
    pub key_matrix: KeyMatrix,
}

#[inline(always)]
pub fn init(_: cortex_m::Peripherals) -> Bsp {
    defmt::info!("BSP init");

    let mut config = embassy_nrf::config::Config::default();
    config.hfclk_source = HfclkSource::ExternalXtal;
    // config.dcdc.reg0 = true;
    let p = embassy_nrf::init(config);

    //
    // Right or left?
    //
    let right_or_left = Input::new(p.P0_09, Pull::Up);
    cortex_m::asm::delay(10_000);

    //
    // Buttons
    //
    let kio0 = p.P0_10.degrade();
    let kio1 = p.P0_17.degrade();
    let kio2 = p.P0_15.degrade();
    let kio3 = p.P0_02.degrade();
    let kio4 = p.P0_05.degrade();
    let kio5 = p.P0_04.degrade();
    let kio6 = p.P0_01.degrade();
    let kio7 = p.P0_30.degrade();
    let kio8 = p.P0_29.degrade();
    let kio9 = p.P0_28.degrade();

    let key_matrix = if right_or_left.is_high() {
        defmt::info!("Right keyboard half detected");

        let rows = [
            Output::new(kio4, Level::High, OutputDrive::Standard),
            Output::new(kio5, Level::High, OutputDrive::Standard),
            Output::new(kio6, Level::High, OutputDrive::Standard),
            Output::new(kio7, Level::High, OutputDrive::Standard),
        ];

        let cols = [
            Input::new(kio0, Pull::Up),
            Input::new(kio1, Pull::Up),
            Input::new(kio2, Pull::Up),
            Input::new(kio3, Pull::Up),
            Input::new(kio9, Pull::Up),
            Input::new(kio8, Pull::Up),
        ];

        Matrix::new(cols, rows).unwrap()
    } else {
        defmt::info!("Left keyboard half detected");

        let rows = [
            Output::new(kio0, Level::High, OutputDrive::Standard),
            Output::new(kio1, Level::High, OutputDrive::Standard),
            Output::new(kio2, Level::High, OutputDrive::Standard),
            Output::new(kio3, Level::High, OutputDrive::Standard),
        ];

        let cols = [
            Input::new(kio4, Pull::Up),
            Input::new(kio5, Pull::Up),
            Input::new(kio6, Pull::Up),
            Input::new(kio7, Pull::Up),
            Input::new(kio8, Pull::Up),
            Input::new(kio9, Pull::Up),
        ];

        Matrix::new(cols, rows).unwrap()
    };

    // Reset pin so it does not draw power.
    drop(right_or_left);

    //
    // Battery measurement
    //
    let mut config = saadc::Config::default();
    config.resolution = saadc::Resolution::_12BIT;
    let mut channel_config = saadc::ChannelConfig::single_ended(saadc::VddhDiv5Input);
    channel_config.time = saadc::Time::_40US;
    channel_config.gain = saadc::Gain::GAIN1_4;

    let battery_voltage = BatteryVoltage {
        adc: Saadc::new(p.SAADC, Irqs, config, [channel_config]),
    };

    //
    // Charger
    //

    let stat = Input::new(p.P0_20, Pull::Up);
    let charger_status = ChargerStatus { stat };

    let systick_token = rtic_monotonics::create_nrf_timer0_monotonic_token!();
    Timer0::start(unsafe { core::mem::transmute(()) }, systick_token);
    defmt::info!("init done");

    Bsp {
        battery_voltage,
        charger_status,
        key_matrix,
    }
}

/// Measure battery voltage.
pub struct BatteryVoltage {
    adc: Saadc<'static, 1>,
}

impl BatteryVoltage {
    pub async fn measure_vbat(&mut self) -> f32 {
        let mut buf = [0; 1];
        self.adc.sample(&mut buf).await;

        (buf[0] as f32 / ((1 << 12) as f32 * (5. / 12.))) * 5.
    }
}
