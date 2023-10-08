use crate::keyboard_app::*;
use corne_firmware::{bsp::keyboard::Mono, radio::Radio, radio_protocol::keyboard_radio_runner};
use keyberon::debounce::Debouncer;
use rtic_monotonics::nrf::timer::ExtU64;

pub async fn battery_handling(cx: battery_handling::Context<'_>) -> ! {
    let bat = cx.local.battery_voltage;
    loop {
        Mono::delay(1.secs()).await;
        let vbat = bat.measure_vbat().await;
        defmt::info!("Vbat = {} V", vbat);
    }
}

pub async fn key_matrix(cx: key_matrix::Context<'_>) -> ! {
    let keys = cx.local.key_matrix;

    let mut events = Debouncer::new([[false; 6]; 4], [[false; 6]; 4], 5);

    loop {
        let keys = keys.get_with_delay(|| cortex_m::asm::delay(20)).unwrap();

        let e = events.events(keys);

        for event in e {
            match event {
                keyberon::layout::Event::Press(i, j) => defmt::info!("Pressed ({},{})", i, j),
                keyberon::layout::Event::Release(i, j) => defmt::info!("Released ({},{})", i, j),
            }
        }

        Mono::delay(1.millis()).await;
    }
}

pub async fn radio_task(_: radio_task::Context<'_>, radio: Radio) -> ! {
    keyboard_radio_runner(radio).await
}
