use super::start_timer0_monotonic;
use crate::radio::Radio;
use embassy_nrf::{
    bind_interrupts,
    config::HfclkSource,
    gpio::{Input, Level, Output, OutputDrive, Pull},
    pac,
    peripherals::{self, P0_00, P0_03},
    rng::{self, Rng},
};

use p256_cortex_m4::{Keypair, PublicKey};
use rtic_monotonics::Monotonic;

pub use super::Mono;

pub type DongleLed = Output<'static, P0_00>;
pub type Button = Input<'static, P0_03>;

pub struct DongleBsp {
    pub led: DongleLed,
    pub button: Button,
    pub radio: Radio,
}

bind_interrupts!(struct Irqs {
    RNG => rng::InterruptHandler<peripherals::RNG>;
});

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

    // Testing crypto
    let n = Mono::now();
    let mut rng = Rng::new(p.RNG, Irqs);
    let d = Mono::now() - n;

    defmt::error!("Key generation took {}", d);

    // On unit A
    let keypair_a = Keypair::random(&mut rng);
    // On unit B
    let keypair_b = Keypair::random(&mut rng);

    let n = Mono::now();
    // On unit A
    let shared_secret1 = keypair_a.secret.agree(&keypair_b.public);
    let a = Mono::now();

    // On unit B
    let shared_secret2 = keypair_b.secret.agree(&keypair_a.public);

    defmt::info!("Pub a: {:x}", keypair_a.public.to_untagged_bytes());
    defmt::info!("Pub b: {:x}", keypair_b.public.to_untagged_bytes());

    defmt::info!("Shared secret 1: {:x}", shared_secret1.as_bytes());
    defmt::info!("Shared secret 2: {:x}", shared_secret2.as_bytes());

    defmt::info!("Time to generate shared secret: {}", a - n);

    let id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10u8];
    let s = Mono::now();
    let signature = keypair_a.secret.sign_prehashed(&id, &mut rng);
    let n = Mono::now();

    let verification = keypair_a.public.verify_prehashed(&id, &signature);
    let a = Mono::now();

    defmt::info!(
        "Signature ({}): {:x}",
        verification,
        signature.to_untagged_bytes()
    );
    defmt::info!("Time to sign: {}", n - s);
    defmt::info!("Time to verify signature: {}", a - n);

    let sec1 = keypair_a.public.to_compressed_sec1_bytes();
    defmt::info!("Compressed public key: {:x}", sec1);
    let maybe_a = PublicKey::from_sec1_bytes(&sec1).unwrap();
    defmt::info!(
        "Pub a: {}",
        maybe_a.to_untagged_bytes() == keypair_a.public.to_untagged_bytes()
    );

    DongleBsp {
        led: Output::new(p.P0_00, Level::Low, OutputDrive::Standard),
        button: Input::new(p.P0_03, Pull::Up),
        radio,
    }
}
