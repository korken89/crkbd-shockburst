use super::start_timer0_monotonic;
use crate::radio::Radio;
use ccm::AeadInPlace;
use embassy_nrf::{
    bind_interrupts,
    config::HfclkSource,
    gpio::{Input, Level, Output, OutputDrive, Pull},
    pac,
    peripherals::{self, P0_00, P0_03},
    rng::{self, Rng},
};

use p256_cortex_m4::{Keypair, PublicKey};
use rand_chacha::rand_core::SeedableRng;
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
    let mut rng = Rng::new(p.RNG, Irqs);
    rng.set_bias_correction(true);

    let mut seed = [0; 32];
    rng.blocking_fill_bytes(&mut seed);
    let mut rng2 = rand_chacha::ChaCha8Rng::from_seed(seed);

    defmt::info!("");

    {
        let n = Mono::now();

        // On unit A
        let keypair_a = Keypair::random(&mut rng);

        let d = Mono::now() - n;
        defmt::error!("Key generation took {}", d);

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
        let signature = keypair_a.secret.sign(&id, &mut rng);
        let n = Mono::now();

        let verification = keypair_a.public.verify(&id, &signature);
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
    }
    defmt::info!("");

    {
        // X25519
        let n = Mono::now();
        let keypair1 = curve25519_cortex_m4::x25519::Keypair::random(&mut rng2);
        let d = Mono::now() - n;

        defmt::info!("generate x25519 key: {}", d);
        defmt::info!("public key: {}", keypair1.public.as_bytes());

        let n = Mono::now();
        let keypair2 = curve25519_cortex_m4::x25519::Keypair::random(&mut rng);
        let d = Mono::now() - n;

        defmt::info!("generate x25519 key2: {}", d);
        defmt::info!("public key: {}", keypair2.public.as_bytes());

        let n = Mono::now();
        let secret1 = keypair1.secret.agree(&keypair2.public);
        let d = Mono::now() - n;
        defmt::info!("generate x25519 secret: {}", d);

        let secret2 = keypair2.secret.agree(&keypair1.public);
        defmt::info!("x25519 secret1: {}", secret1.as_bytes());
        defmt::info!("x25519 secret2: {}", secret2.as_bytes());
    }
    defmt::info!("");

    {
        use salty::agreement;
        // salty
        let mut seed = [0; 32];
        rng.blocking_fill_bytes(&mut seed);

        let n = Mono::now();
        let secret1 = agreement::SecretKey::from_seed(&seed);
        let public1 = secret1.public();
        let d = Mono::now() - n;

        defmt::info!("generate x25519 key: {}", d);
        defmt::info!("public key: {}", public1.to_bytes());

        rng.blocking_fill_bytes(&mut seed);

        let secret2 = agreement::SecretKey::from_seed(&seed);
        let public2 = secret2.public();
        defmt::info!("public key: {}", public2.to_bytes());

        let n = Mono::now();
        let secret1 = secret1.agree(&public2);
        let d = Mono::now() - n;
        defmt::info!("generate x25519 secret: {}", d);

        let secret2 = secret2.agree(&public1);
        defmt::info!("x25519 secret1: {}", secret1.to_bytes());
        defmt::info!("x25519 secret2: {}", secret2.to_bytes());
    }
    defmt::info!("");

    // Test AES
    // use aes::cipher::{
    //     generic_array::GenericArray, BlockCipher, BlockDecrypt, BlockEncrypt, KeyInit,
    // };
    // use aes::Aes128;

    // let key = GenericArray::from([0u8; 16]);
    // let mut block = GenericArray::from([42u8; 16]);

    // // Initialize cipher
    // let cipher = Aes128::new(&key);

    // // Encrypt block in-place
    // let n = Mono::now();
    // cipher.encrypt_block(&mut block);
    // let d = Mono::now() - n;

    // defmt::info!("Aes encrypt: {}", d);
    // defmt::info!("Aes encrypt: {}", block.as_slice());

    // // And decrypt it back
    // let n = Mono::now();
    // cipher.decrypt_block(&mut block);
    // let d = Mono::now() - n;

    // defmt::info!("Aes decrypt: {}", d);
    {
        use aes::Aes128;
        use ccm::{
            aead::{generic_array::GenericArray, KeyInit},
            consts::{U10, U13},
            Ccm,
        };

        // AES-256-CCM type with tag and nonce size equal to 10 and 13 bytes respectively
        pub type Aes128Ccm = Ccm<Aes128, U10, U13>;

        let key = Aes128Ccm::generate_key(&mut rng);
        let cipher = Aes128Ccm::new(&key);
        let nonce = GenericArray::from_slice(b"unique nonce."); // 13-bytes; unique per message

        let mut block = ccm::aead::heapless::Vec::<u8, 128>::new();
        block.extend_from_slice(&[42; 64]).ok();

        let n = Mono::now();
        cipher.encrypt_in_place(&nonce, b"", &mut block).ok();
        let d = Mono::now() - n;

        defmt::info!("Aes encrypt: {}", d);
        defmt::info!("Aes encrypt (len = {}): {}", block.len(), block.as_slice());

        // And decrypt it back
        let n = Mono::now();
        cipher.decrypt_in_place(&nonce, b"", &mut block).ok();
        let d = Mono::now() - n;

        defmt::info!("Aes decrypt: {}", d);
    }

    defmt::info!("");
    // Test chacha
    {
        use chacha20poly1305::{
            aead::{heapless, AeadCore, KeyInit},
            ChaCha8Poly1305,
        };

        let key = ChaCha8Poly1305::generate_key(&mut rng);
        let cipher = ChaCha8Poly1305::new(&key);
        let nonce = ChaCha8Poly1305::generate_nonce(&mut rng); // 96-bits; unique per message

        let mut block = heapless::Vec::<u8, 128>::new();
        block.extend_from_slice(&[42; 64]).ok();

        let n = Mono::now();
        cipher.encrypt_in_place(&nonce, b"", &mut block).ok();
        let d = Mono::now() - n;

        defmt::info!("Chacha encrypt: {}", d);
        defmt::info!(
            "Chacha encrypt (len = {}): {}",
            block.len(),
            block.as_slice()
        );

        // And decrypt it back
        let n = Mono::now();
        cipher.decrypt_in_place(&nonce, b"", &mut block).ok();
        let d = Mono::now() - n;

        defmt::info!("Chacha decrypt: {}", d);
    }

    defmt::info!("");
    // let ciphertext = cipher.encrypt(&nonce, b"plaintext message".as_ref())?;
    // let plaintext = cipher.decrypt(&nonce, ciphertext.as_ref())?;

    // let ciphertext = cipher.encrypt(nonce, b"plaintext message".as_ref())?;
    // let plaintext = cipher.decrypt(nonce, ciphertext.as_ref())?;

    DongleBsp {
        led: Output::new(p.P0_00, Level::Low, OutputDrive::Standard),
        button: Input::new(p.P0_03, Pull::Up),
        radio,
    }
}
