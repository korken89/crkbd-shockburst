use crate::app::*;
use keyberon::debounce::Debouncer;
use rtic_monotonics::{nrf::timer::*, Monotonic};

pub async fn task(cx: task::Context<'_>) -> ! {
    let keys = cx.local.key_matrix;
    let bat = cx.local.battery_voltage;

    let mut events = Debouncer::new([[false; 6]; 4], [[false; 6]; 4], 5);

    loop {
        // let vbat = bat.measure_vbat().await;
        // defmt::info!("Vbat = {} V", vbat);

        let keys = keys.get_with_delay(|| cortex_m::asm::delay(20)).unwrap();

        let e = events.events(keys);

        for event in e {
            match event {
                keyberon::layout::Event::Press(i, j) => defmt::info!("Pressed ({},{})", i, j),
                keyberon::layout::Event::Release(i, j) => defmt::info!("Released ({},{})", i, j),
            }
        }

        // defmt::info!("Scan time = {}", diff);
        // for col in keys {
        //     defmt::info!("{}", col);
        // }

        Timer0::delay(1.millis()).await;
    }
}
