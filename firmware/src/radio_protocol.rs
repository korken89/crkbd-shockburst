//! Radio communication
//!
//! 1. The dongle will be sending "sync" frames every 100 rounds, this is when we are at a known channel.
//!     - All messages in each frame will be frequency hopping according to a known pattern.
//! 2. After sync is received, the keyboard halves will send their state in predetermined slots.
//!     - Each slot will be X ms, where each X is the right half's and every X+1 is the left's.
//!     - If there has been a state change in the keyboard input, the new full state will be sent.
//!     - It will be sent, expecting an ACK from the dongle.
//!     - If no ACK is received, the state will be retransmitted until an ACK is received, or
//!       until the keyboard gets a new state.
//!     - If there is no new data for a full frame, the keyboard will send out its state anyways.

use crate::bsp::dongle::DongleLed;
use crate::bsp::Mono;
use crate::radio::{Packet, Radio};
use rtic_monotonics::nrf::timer::fugit::TimerInstantU64;
use rtic_monotonics::{nrf::timer::*, Monotonic};

/// A channel hopping selector implementation.
pub struct ChannelHopping {
    state: u8,
}

impl ChannelHopping {
    // Randomly generated at: https://www.random.org/sequences/?min=1&max=100&col=1&format=html&rnd=new
    // and slightly modified to make adjacent hops not be adjacent channels.
    const CHANNEL_HOPPING_SEQUENCE: [u8; 100] = [
        47, 84, 37, 45, 74, 13, 44, 75, 67, 28, 65, 51, 68, 7, 89, 9, 16, 63, 8, 87, 23, 99, 57,
        69, 12, 26, 83, 30, 78, 97, 33, 77, 41, 34, 86, 42, 70, 95, 6, 73, 88, 2, 72, 59, 4, 25,
        53, 96, 20, 5, 39, 92, 82, 71, 29, 43, 1, 94, 32, 17, 60, 90, 56, 27, 11, 55, 62, 79, 98,
        64, 14, 52, 100, 93, 76, 46, 85, 58, 18, 3, 15, 40, 10, 19, 48, 61, 80, 36, 54, 21, 81, 38,
        22, 49, 91, 31, 66, 50, 35, 24,
    ];

    /// Create a new channel hopping selector.
    pub const fn new() -> Self {
        Self { state: 0 }
    }

    /// Get the current channel.
    pub fn current_channel(&self) -> u8 {
        Self::CHANNEL_HOPPING_SEQUENCE[self.state as usize]
    }

    /// Move to the next channel.
    pub fn next_channel(&mut self) {
        self.state = (self.state + 1) % Self::CHANNEL_HOPPING_SEQUENCE.len() as u8;
    }

    /// Check if the current channel is the initial state.
    pub fn is_initial_state(&self) -> bool {
        self.state == 0
    }

    /// Reset to initial state.
    pub fn reset(&mut self) {
        self.state = 0;
    }

    /// Return the current timeslot.
    pub fn state(&self) -> u8 {
        self.state
    }
}

/// Main runner for the dongle's radio communication.
pub async fn dongle_radio_runner(mut radio: Radio) -> ! {
    let mut packet = Packet::new();
    let mut slot_start_time = Mono::now() + 200.millis();
    let mut channel_hopping = ChannelHopping::new();

    loop {
        //
        // 1. Send the sync packet at the desired time.
        //
        Mono::delay_until(slot_start_time).await;

        defmt::info!(
            "Trying to send on channel {} ({}) at {}...",
            channel_hopping.current_channel(),
            channel_hopping.state,
            slot_start_time
        );

        radio.set_freqeuency(channel_hopping.current_channel());
        // TODO: Actually send something as sync
        packet.copy_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let sync_timestamp = radio.send(&mut packet).await.0;

        //
        // 2. Receive and channel hop, look for keyboard halves responses.
        //
        channel_hopping.next_channel();
        slot_start_time += 2.millis();

        while !channel_hopping.is_initial_state() {
            radio.set_freqeuency(channel_hopping.current_channel());

            // Look for packets, stop receiving a little before the next round.
            match Mono::timeout_at(slot_start_time + 1800.micros(), radio.recv(&mut packet)).await {
                Ok(ts) => {
                    if let Ok((ts, rssi)) = ts {
                        defmt::debug!("Got data, channel {}: {}", channel_hopping.state(), *packet,);

                        // TODO: Send ack.
                        packet.copy_from_slice(&[10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
                        radio.send_no_cca(&mut packet).await;
                    }
                }
                Err(_timeout) => {
                    defmt::trace!("No data, channel {}", channel_hopping.current_channel())
                }
            };

            channel_hopping.next_channel();
            slot_start_time += 2.millis(); // 2 ms per RX slot
        }
    }
}

#[derive(Copy, Clone, Debug, defmt::Format)]
enum KeyboardRadioState {
    LookingForSync,
    Synchronized {
        sync_time: TimerInstantU64<1_000_000>,
        slot_start_time: TimerInstantU64<1_000_000>,
    },
}

/// Main runner for a keyboard half's radio communication.
pub async fn keyboard_radio_runner(mut radio: Radio, is_right_half: bool) -> ! {
    let mut packet = Packet::new();
    let mut channel_hopping = ChannelHopping::new();

    let mut state = KeyboardRadioState::LookingForSync;

    // RX code:
    loop {
        // if led.is_set_high() {
        //     led.set_low();
        // } else {
        //     led.set_high();
        // }

        match state {
            KeyboardRadioState::LookingForSync => {
                channel_hopping.reset();
                radio.set_freqeuency(channel_hopping.current_channel());
                let (timestamp, rssi) = if let Ok(v) = radio.recv(&mut packet).await {
                    v
                } else {
                    defmt::info!("Radio receive error");
                    continue;
                };

                defmt::info!(
                    "Radio receive ts {}, rssi {}, packet: {}",
                    timestamp,
                    rssi,
                    *packet
                );

                if channel_hopping.is_initial_state() && &*packet == [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
                {
                    defmt::error!("Sync found at {}", timestamp.0);

                    // Hack to get RX timestamp in mono time...
                    let now = TimerInstantU64::from_ticks(
                        Mono::now().ticks() & 0xffff_ffff_0000_0000 | timestamp.0.ticks() as u64,
                    );

                    if is_right_half {
                        // Right half gets the odd slots.
                        channel_hopping.next_channel();
                        state = KeyboardRadioState::Synchronized {
                            sync_time: now,
                            slot_start_time: now + 2000.micros(),
                        };
                    } else {
                        // Left half gets the even slots.
                        channel_hopping.next_channel();
                        channel_hopping.next_channel();
                        state = KeyboardRadioState::Synchronized {
                            sync_time: now,
                            slot_start_time: now + 4000.micros(),
                        };
                    }
                }
            }
            KeyboardRadioState::Synchronized {
                sync_time,
                mut slot_start_time,
            } => {
                loop {
                    radio.set_freqeuency(channel_hopping.current_channel());

                    // TODO: Send data and wait for ack.
                    packet.copy_from_slice(&[channel_hopping.state()]);

                    Mono::delay_until(slot_start_time).await;
                    let timestamp = radio.send(&mut packet).await;

                    defmt::info!(
                        "Sent at {}, sync = {}, diff = {} ms",
                        timestamp.0,
                        slot_start_time,
                        (slot_start_time - sync_time).to_millis(),
                    );

                    // Look for ACK.
                    match Mono::timeout_at(slot_start_time + 1800.micros(), radio.recv(&mut packet))
                        .await
                    {
                        Ok(_) => {
                            defmt::info!("Got ack, channel {}", channel_hopping.current_channel());
                        }
                        Err(_timeout) => {
                            defmt::warn!("No ack, channel {}", channel_hopping.current_channel(),)
                        }
                    };

                    slot_start_time += 4.millis();

                    // Jump 2 channels as every keyboard half gets half of the slots.
                    channel_hopping.next_channel();
                    if channel_hopping.is_initial_state() {
                        break;
                    }

                    channel_hopping.next_channel();
                    if channel_hopping.is_initial_state() {
                        break;
                    }
                }

                state = KeyboardRadioState::LookingForSync;
            }
        }
    }
}
