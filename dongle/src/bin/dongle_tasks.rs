use crate::app::*;
use corne::bsp::radio::Packet;
use rtic_monotonics::{
    nrf::timer::{
        fugit::{Instant, TimerDurationU64, TimerInstantU32},
        *,
    },
    Monotonic,
};

// Radio communication
//
// 1. The dongle will be sending "sync" frames at 1 Hz, this is when we are at a known channel
//     - All messages in each frame will be frequency hopping according to a known pattern
// 2. After sync is received, the keyboard halves will send their state in predetermined slots
//     - Each slot will be 1 ms, where each even ms is the right half's and every odd is the left's
//     - If there has been a state change in the keyboard input, the new full state will be sent
//     - It will be sent, expecting an ACK from the dongle
//     - If no ACK is received, the state will be retransmitted until an ACK is received
//     - If there is no new data for a full frame, the keyboard will send out its state anyways

fn lfsr(state: &mut u8) {
    let lfsr = *state;
    let bit = ((lfsr >> 0) ^ (lfsr >> 1)) & 1;
    *state = (lfsr >> 1) | (bit << 5);
}

pub async fn radio_task(cx: radio_task::Context<'_>) -> ! {
    let radio = cx.local.radio;
    let led = cx.local.led;

    let mut packet = Packet::new();

    // // RX code:
    // loop {
    //     Timer0::delay(100.millis()).await;

    //     if led.is_set_high() {
    //         led.set_low();
    //     } else {
    //         led.set_high();
    //     }

    //     let r = radio.recv(&mut packet).await;

    //     defmt::info!("Radio receive finished with {}, packet: {}", r, *packet);
    // }

    // TX code:
    packet.copy_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    let send_dt = 200.millis();

    let mut desired_send_time = Timer0::now() + send_dt;
    let mut compensation = 0.;
    let mut gain = 1.;

    let mut channel = 1;

    loop {
        Timer0::delay_until(
            desired_send_time - TimerDurationU64::<1_000_000>::from_ticks(compensation as u64),
        )
        .await;
        if led.is_set_high() {
            led.set_low();
        } else {
            led.set_high();
        }

        defmt::info!("Trying to send...");
        //let start = Timer0::now();
        let timestamp = radio.send(&mut packet).await.0;

        // Look for ACK (TODO: Until the end of slot - guard time)
        match Timer0::timeout_after(1300.micros(), radio.recv(&mut packet)).await {
            Ok(_) => defmt::info!("Got ack!"),
            Err(_timeout) => defmt::info!("No ack..."),
        };

        //let end = Timer0::now();
        defmt::info!("Packet sent! TX at {}", timestamp);

        let dst = TimerInstantU32::<1_000_000>::from_ticks(
            desired_send_time.ticks() as u32 & 0xffff_ffff,
        );
        if let Some(dur) = timestamp.checked_duration_since(dst) {
            defmt::info!("Packet send {} too late", dur);
            compensation += dur.ticks() as f32 * gain;
        } else if let Some(dur) = dst.checked_duration_since(timestamp) {
            defmt::info!("Packet send {} too early", dur);
            compensation -= dur.ticks() as f32 * gain;
        } else {
            defmt::error!("wat");
        }

        if gain > 0.05 {
            gain *= 0.9;
        }

        defmt::info!("gain {}, comp {}, channel {}", gain, compensation, channel);

        desired_send_time += send_dt;
        lfsr(&mut channel);
    }
}
