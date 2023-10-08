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

pub struct ChannelHopping {
    state: u8,
}

impl ChannelHopping {
    // Randomly generated at: https://www.random.org/sequences/?min=1&max=100&col=1&format=html&rnd=new
    // and slightly modified to make adjacent hops not be adjacent channels.
    const CHANNEL_HOPPING_SEQUENCE: [u8; 100] = [
        84, 47, 37, 45, 74, 44, 13, 75, 67, 28, 65, 51, 68, 7, 89, 9, 16, 63, 8, 87, 23, 99, 57,
        69, 12, 26, 83, 30, 78, 33, 97, 77, 41, 34, 42, 86, 70, 95, 6, 73, 88, 2, 72, 59, 4, 25,
        53, 96, 20, 5, 39, 92, 82, 71, 29, 43, 1, 94, 32, 17, 60, 90, 56, 27, 11, 55, 62, 79, 64,
        98, 14, 52, 100, 93, 76, 46, 85, 58, 18, 3, 15, 40, 48, 10, 19, 61, 54, 80, 36, 21, 81, 38,
        22, 49, 91, 31, 66, 35, 24, 50,
    ];

    pub const fn new() -> Self {
        Self { state: 0 }
    }

    pub fn current_channel(&self) -> u8 {
        Self::CHANNEL_HOPPING_SEQUENCE[self.state as usize]
    }

    pub fn next_channel(&mut self) {
        self.state = (self.state + 1) % Self::CHANNEL_HOPPING_SEQUENCE.len() as u8;
    }

    pub fn is_initial_state(&self) -> bool {
        self.state == 0
    }
}

struct TxCompensation {}

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

    // let send_dt = 200.millis();

    let mut desired_time = Timer0::now() + 200.millis();
    let mut compensation = 0.;
    let mut gain = 1.;

    let mut channel = ChannelHopping::new();

    loop {
        //
        // 1. Send the sync packet at the desired time.
        //
        Timer0::delay_until(
            desired_time - TimerDurationU64::<1_000_000>::from_ticks(compensation as u64),
        )
        .await;
        if led.is_set_high() {
            led.set_low();
        } else {
            led.set_high();
        }

        let current_channel = channel.current_channel();
        defmt::info!(
            "Trying to send on channel {} ({}) at {}...",
            current_channel,
            channel.state,
            desired_time
        );

        radio.set_freqeuency(current_channel);
        //let start = Timer0::now();
        packet.copy_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let timestamp = radio.send(&mut packet).await.0;

        // // Compensate next TX round. (this is for keyboard halves to make sure they hit the packet slot, if needed)
        // let dst =
        //     TimerInstantU32::<1_000_000>::from_ticks(desired_time.ticks() as u32 & 0xffff_ffff);
        // if let Some(dur) = timestamp.checked_duration_since(dst) {
        //     defmt::info!("Packet send {} too late", dur);
        //     compensation += dur.ticks() as f32 * gain;
        // } else if let Some(dur) = dst.checked_duration_since(timestamp) {
        //     defmt::info!("Packet send {} too early", dur);
        //     compensation -= dur.ticks() as f32 * gain;
        // }

        // if gain > 0.05 {
        //     gain *= 0.9;
        // }

        //
        // Receive and channel hop.
        //
        channel.next_channel();
        desired_time += 2.millis();

        while !channel.is_initial_state() {
            // Look for packets.
            match Timer0::timeout_at(desired_time - 100.micros(), radio.recv(&mut packet)).await {
                Ok(_) => {
                    defmt::debug!("Got data, channel {}", channel.current_channel());

                    // TODO: Send ack.
                    packet.copy_from_slice(&[10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
                    radio.send_no_cca(&mut packet).await;
                }
                Err(_timeout) => defmt::trace!("No data, channel {}", channel.current_channel()),
            };

            channel.next_channel();
            desired_time += 2.millis();
        }

        //let end = Timer0::now();
        // defmt::info!("Packet sent! TX at {}", timestamp);

        // defmt::info!(
        //     "gain {}, comp {}, channel {}",
        //     gain,
        //     compensation,
        //     channel.current_channel()
        // );
    }
}
