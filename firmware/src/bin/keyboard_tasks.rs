use crate::keyboard_app::*;
use corne_firmware::{bsp::keyboard::Mono, radio::Radio, radio_protocol::keyboard_radio_runner};
use keyberon::{debounce::Debouncer, layout::Event};
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

        if events.update(keys) {
            let new = pack_bools(events.get());

            // TODO: Send an update
        }

        // let e = events.events(keys);

        // for event in e {
        //     match event {
        //         Event::Press(i, j) => defmt::info!("Pressed ({},{})", i, j),
        //         Event::Release(i, j) => defmt::info!("Released ({},{})", i, j),
        //     }
        // }

        Mono::delay(1.millis()).await;
    }
}

pub async fn radio_task(_: radio_task::Context<'_>, radio: Radio, is_right_half: bool) -> ! {
    keyboard_radio_runner(radio).await
}

#[inline(always)]
fn pack_bools(bools: &[[bool; 6]; 4]) -> [u8; 3] {
    let mut state: u32 = 0;

    for b2 in bools {
        for b1 in b2 {
            state <<= 1;
            state |= *b1 as u32;
        }
    }

    state.to_le_bytes()[0..3].try_into().unwrap()
}
