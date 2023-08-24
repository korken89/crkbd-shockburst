use crate::app::*;
use corne::bsp::radio::Packet;
use rtic_monotonics::nrf::timer::*;

pub async fn radio_task(cx: radio_task::Context<'_>) -> ! {
    let radio = cx.local.radio;
    let led = cx.local.led;

    let mut packet = Packet::new();

    loop {
        Timer0::delay(100.millis()).await;

        if led.is_set_high() {
            led.set_low();
        } else {
            led.set_high();
        }

        let r = radio.recv(&mut packet);
        let lqi = packet.lqi();

        defmt::info!(
            "Radio receive finished with {} (RSSI = -{} dBm), packet: {}",
            r,
            lqi,
            *packet
        );
    }

    // TX code:
    // packet.copy_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    // loop {
    //     Timer0::delay(500.millis()).await;
    //     if led.is_set_high() {
    //         led.set_low();
    //     } else {
    //         led.set_high();
    //     }

    //     defmt::info!("Trying to send...");
    //     radio.send(&mut packet);
    //     defmt::info!("Packet sent!");
    // }
}
