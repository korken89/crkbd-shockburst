use crate::dongle_app::*;
use corne_firmware::{radio::Radio, radio_protocol::dongle_radio_runner};

pub async fn radio_task(_: radio_task::Context<'_>, radio: Radio) -> ! {
    dongle_radio_runner(radio).await
}

// OLD CODE

// let led = cx.local.led;
// let mut packet = Packet::new();
// // RX code:
// loop {
//     Mono::delay(100.millis()).await;

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

// let mut desired_time = Mono::now() + 200.millis();
// let mut compensation = 0.;
// let mut gain = 1.;

// let mut channel = ChannelHopping::new();

// loop {
//     //
//     // 1. Send the sync packet at the desired time.
//     //
//     Mono::delay_until(
//         desired_time - TimerDurationU64::<1_000_000>::from_ticks(compensation as u64),
//     )
//     .await;
//     if led.is_set_high() {
//         led.set_low();
//     } else {
//         led.set_high();
//     }

//     let current_channel = channel.current_channel();
//     defmt::info!(
//         "Trying to send on channel {} at {}...",
//         current_channel,
//         desired_time
//     );

//     radio.set_freqeuency(current_channel);
//     //let start = Mono::now();
//     packet.copy_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
//     let timestamp = radio.send(&mut packet).await.0;

//     // // Compensate next TX round. (this is for keyboard halves to make sure they hit the packet slot, if needed)
//     // let dst =
//     //     TimerInstantU32::<1_000_000>::from_ticks(desired_time.ticks() as u32 & 0xffff_ffff);
//     // if let Some(dur) = timestamp.checked_duration_since(dst) {
//     //     defmt::info!("Packet send {} too late", dur);
//     //     compensation += dur.ticks() as f32 * gain;
//     // } else if let Some(dur) = dst.checked_duration_since(timestamp) {
//     //     defmt::info!("Packet send {} too early", dur);
//     //     compensation -= dur.ticks() as f32 * gain;
//     // }

//     // if gain > 0.05 {
//     //     gain *= 0.9;
//     // }

//     //
//     // Receive and channel hop.
//     //
//     channel.next_channel();
//     desired_time += 2.millis();

//     while !channel.is_initial_state() {
//         // Look for packets.
//         match Mono::timeout_at(desired_time - 100.micros(), radio.recv(&mut packet)).await {
//             Ok(_) => {
//                 defmt::debug!("Got data, channel {}", channel.current_channel());

//                 // TODO: Send ack.
//                 packet.copy_from_slice(&[10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
//                 radio.send_no_cca(&mut packet).await;
//             }
//             Err(_timeout) => defmt::trace!("No data, channel {}", channel.current_channel()),
//         };

//         channel.next_channel();
//         desired_time += 2.millis();
//     }

//let end = Mono::now();
// defmt::info!("Packet sent! TX at {}", timestamp);

// defmt::info!(
//     "gain {}, comp {}, channel {}",
//     gain,
//     compensation,
//     channel.current_channel()
// );
