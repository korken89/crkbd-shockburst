//! IEEE 802.15.4 radio

use super::RadioTimestamps;
use crate::waker_registration::CriticalSectionWakerRegistration;
use core::{
    ops::{self, RangeFrom},
    sync::atomic::{self, Ordering},
    task::Poll,
};
use cortex_m::peripheral::NVIC;
use embassy_nrf::pac::{
    self,
    radio::{state::STATE_A, txpower::TXPOWER_A},
    Interrupt, RADIO,
};
use rtic_monotonics::nrf::timer::fugit::{TimerDurationU32, TimerInstantU32};

struct OnDrop<F: FnOnce()> {
    f: core::mem::MaybeUninit<F>,
}

impl<F: FnOnce()> OnDrop<F> {
    pub fn new(f: F) -> Self {
        Self {
            f: core::mem::MaybeUninit::new(f),
        }
    }

    pub fn defuse(self) {
        core::mem::forget(self)
    }
}

impl<F: FnOnce()> Drop for OnDrop<F> {
    fn drop(&mut self) {
        unsafe { self.f.as_ptr().read()() }
    }
}

/// IEEE 802.15.4 radio
pub struct Radio {
    radio: RADIO,
    // RADIO needs to be (re-)enabled to pick up new settings
    needs_enable: bool,
}

/// Timestamp for when the `address` portion of the packet was sent or received.
#[derive(Copy, Clone, Debug, defmt::Format, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub TimerInstantU32<1_000_000>);

/// RSSI value in dBm.
#[derive(Copy, Clone, Debug, defmt::Format, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rssi(pub i8);

static WAKER: CriticalSectionWakerRegistration = CriticalSectionWakerRegistration::new();

// Bind the radio interrupt.
#[no_mangle]
#[allow(non_snake_case)]
unsafe extern "C" fn RADIO() {
    let radio = unsafe { &*pac::RADIO::PTR };

    // We got an event, clear interrupts and wake the waker.
    radio.intenclr.write(|w| w.bits(0xffffffff));

    defmt::trace!("RADIO IRQ");

    WAKER.wake()
}

/// Default Clear Channel Assessment method = Carrier sense
pub const DEFAULT_CCA: Cca = Cca::CarrierSense;

/// Default radio channel = Channel 11 (`2_405` MHz)
pub const DEFAULT_CHANNEL: Channel = Channel::_11;

/// Default TX power = 0 dBm
pub const DEFAULT_TXPOWER: TxPower = TxPower::_0dBm;

/// Default Start of Frame Delimiter = `0xA7` (IEEE compliant)
pub const DEFAULT_SFD: u8 = 0xA7;

// TODO expose the other variants in `pac::CCAMODE_A`
/// Clear Channel Assessment method
pub enum Cca {
    /// Carrier sense
    CarrierSense,
    /// Energy Detection / Energy Above Threshold
    EnergyDetection {
        /// Energy measurements above this value mean that the channel is assumed to be busy.
        /// Note the the measurement range is 0..0xFF - where 0 means that the received power was
        /// less than 10 dB above the selected receiver sensitivity. This value is not given in dBm,
        /// but can be converted. See the nrf52840 Product Specification Section 6.20.12.4
        /// for details.
        ed_threshold: u8,
    },
}

/// IEEE 802.15.4 channels
///
/// NOTE these are NOT the same as WiFi 2.4 GHz channels
pub enum Channel {
    /// 2_405 MHz
    _11 = 5,
    /// 2_410 MHz
    _12 = 10,
    /// 2_415 MHz
    _13 = 15,
    /// 2_420 MHz
    _14 = 20,
    /// 2_425 MHz
    _15 = 25,
    /// 2_430 MHz
    _16 = 30,
    /// 2_435 MHz
    _17 = 35,
    /// 2_440 MHz
    _18 = 40,
    /// 2_445 MHz
    _19 = 45,
    /// 2_450 MHz
    _20 = 50,
    /// 2_455 MHz
    _21 = 55,
    /// 2_460 MHz
    _22 = 60,
    /// 2_465 MHz
    _23 = 65,
    /// 2_470 MHz
    _24 = 70,
    /// 2_475 MHz
    _25 = 75,
    /// 2_480 MHz
    _26 = 80,
}

/// Transmission power in dBm (decibel milliwatt)
// TXPOWERA enum minus the deprecated Neg30dBm variant and with better docs
#[derive(Clone, Copy, PartialEq)]
pub enum TxPower {
    /// +8 dBm
    Pos8dBm,
    /// +7 dBm
    Pos7dBm,
    /// +6 dBm (~4 mW)
    Pos6dBm,
    /// +5 dBm
    Pos5dBm,
    /// +4 dBm
    Pos4dBm,
    /// +3 dBm (~2 mW)
    Pos3dBm,
    /// +2 dBm
    Pos2dBm,
    /// 0 dBm (1 mW)
    _0dBm,
    /// -4 dBm
    Neg4dBm,
    /// -8 dBm
    Neg8dBm,
    /// -12 dBm
    Neg12dBm,
    /// -16 dBm
    Neg16dBm,
    /// -20 dBm (10 μW)
    Neg20dBm,
    /// -40 dBm (0.1 μW)
    Neg40dBm,
}

impl TxPower {
    fn _into(self) -> TXPOWER_A {
        match self {
            TxPower::Neg40dBm => TXPOWER_A::NEG40D_BM,
            TxPower::Neg20dBm => TXPOWER_A::NEG20D_BM,
            TxPower::Neg16dBm => TXPOWER_A::NEG16D_BM,
            TxPower::Neg12dBm => TXPOWER_A::NEG12D_BM,
            TxPower::Neg8dBm => TXPOWER_A::NEG8D_BM,
            TxPower::Neg4dBm => TXPOWER_A::NEG4D_BM,
            TxPower::_0dBm => TXPOWER_A::_0D_BM,
            TxPower::Pos2dBm => TXPOWER_A::POS2D_BM,
            TxPower::Pos3dBm => TXPOWER_A::POS3D_BM,
            TxPower::Pos4dBm => TXPOWER_A::POS4D_BM,
            TxPower::Pos5dBm => TXPOWER_A::POS5D_BM,
            TxPower::Pos6dBm => TXPOWER_A::POS6D_BM,
            TxPower::Pos7dBm => TXPOWER_A::POS7D_BM,
            TxPower::Pos8dBm => TXPOWER_A::POS8D_BM,
        }
    }
}

impl Radio {
    /// Initializes the radio for IEEE 802.15.4 operation
    pub fn init(radio: RADIO) -> Self {
        let mut radio = Self {
            needs_enable: false,
            radio,
        };

        // shortcuts will be kept off by default and only be temporarily enabled within blocking
        // functions
        radio.radio.shorts.reset();

        // go to a known state
        radio.disable();

        // clear any event of interest to us
        radio.radio.events_disabled.reset();
        radio.radio.events_end.reset();
        radio.radio.events_phyend.reset();
        radio.radio.events_address.reset();
        radio.radio.events_ready.reset();

        radio.radio.mode.write(|w| w.mode().nrf_2mbit());

        let base0 = [0xE7, 0xE7, 0xE7, 0xE7];
        let base1 = [0xC2, 0xC2, 0xC2, 0xC2];
        let prefix0 = [0xE7, 0xC2, 0xC3, 0xC4];
        let prefix1 = [0xC5, 0xC6, 0xC7, 0xC8];

        radio
            .radio
            .base0
            .write(|w| unsafe { w.bits(u32::from_le_bytes(base0)) });
        radio
            .radio
            .base1
            .write(|w| unsafe { w.bits(u32::from_le_bytes(base1)) });

        radio
            .radio
            .prefix0
            .write(|w| unsafe { w.bits(u32::from_le_bytes(prefix0)) });
        radio
            .radio
            .prefix1
            .write(|w| unsafe { w.bits(u32::from_le_bytes(prefix1)) });

        // NOTE(unsafe) radio is currently disabled
        unsafe {
            radio.radio.pcnf0.write(|w| {
                w.s1incl()
                    .clear_bit() // S1 not included in RAM
                    .plen()
                    ._8bit()
                    .crcinc()
                    .include() // the LENGTH field (the value) also accounts for the CRC (2 bytes)
                    .cilen()
                    .bits(0) // no code indicator
                    .lflen()
                    .bits(7) // length = 8 bits (but highest bit is reserved and must be `0`)
                    .s0len()
                    .clear_bit() // no S0
                    .s1len()
                    .bits(0) // no S1
            });

            radio.radio.pcnf1.write(|w| {
                w.maxlen()
                    .bits(Packet::MAX_PSDU_LEN) // payload length
                    .statlen()
                    .bits(0) // no static length
                    .balen()
                    .bits(4) // no base address
                    .endian()
                    .clear_bit() // little endian
                    .whiteen()
                    .clear_bit() // no data whitening
            });

            // Fast ramp-up
            radio.radio.modecnf0.modify(|_, w| w.ru().fast());

            // CRC configuration required by the IEEE spec: x**16 + x**12 + x**5 + 1
            radio.radio.crccnf.write(|w| w.len().two());
            radio.radio.crcpoly.write(|w| w.crcpoly().bits(0x11021));
            radio.radio.crcinit.write(|w| w.crcinit().bits(0));
        }

        // set default settings
        radio.set_channel(DEFAULT_CHANNEL);
        radio.set_cca(DEFAULT_CCA);
        radio.set_sfd(DEFAULT_SFD);
        radio.set_txpower(DEFAULT_TXPOWER);

        // Enable the interrupt
        unsafe {
            //:set_prio(pac::NVIC_PRIO_BITS, Interrupt::$timer);
            NVIC::unmask(Interrupt::RADIO);
        }

        radio
    }

    /// Changes the radio channel
    pub fn set_channel(&mut self, channel: Channel) {
        self.needs_enable = true;
        unsafe {
            self.radio
                .frequency
                .write(|w| w.map().clear_bit().frequency().bits(channel as u8))
        }
    }

    /// Changes the radio frequency in 2400 MHz + `val` where `val = 0..=100`.
    pub fn set_freqeuency(&mut self, frequency: u8) {
        if frequency > 100 {
            panic!("Invalid frequency setting");
        }

        self.needs_enable = true;
        unsafe {
            self.radio
                .frequency
                .write(|w| w.map().clear_bit().frequency().bits(frequency))
        }
    }

    /// Changes the Clear Channel Assessment method
    pub fn set_cca(&mut self, cca: Cca) {
        self.needs_enable = true;
        match cca {
            Cca::CarrierSense => self.radio.ccactrl.write(|w| w.ccamode().carrier_mode()),
            Cca::EnergyDetection { ed_threshold } => {
                // "[ED] is enabled by first configuring the field CCAMODE=EdMode in CCACTRL
                // and writing the CCAEDTHRES field to a chosen value."
                self.radio
                    .ccactrl
                    .write(|w| unsafe { w.ccamode().ed_mode().ccaedthres().bits(ed_threshold) });
            }
        }
    }

    /// Changes the Start of Frame Delimiter
    pub fn set_sfd(&mut self, sfd: u8) {
        // self.needs_enable = true; // this appears to not be needed
        self.radio.sfd.write(|w| unsafe { w.sfd().bits(sfd) });
    }

    /// Changes the TX power
    pub fn set_txpower(&mut self, power: TxPower) {
        self.needs_enable = true;
        self.radio
            .txpower
            .write(|w| w.txpower().variant(power._into()));
    }

    /// Receives one radio packet and copies its contents into the given `packet` buffer
    ///
    /// This methods returns the `Ok` variant if the CRC included the packet was successfully
    /// validated by the hardware; otherwise it returns the `Err` variant. In either case, `packet`
    /// will be updated with the received packet's data
    pub async fn recv(&mut self, packet: &mut Packet) -> Result<(Timestamp, Rssi), u16> {
        // Start the read
        // NOTE(unsafe) We block until reception completes or errors
        unsafe {
            self.start_recv(packet);
        }

        let dropper = OnDrop::new(|| Self::cancel_recv());

        // wait until we have received something
        core::future::poll_fn(|cx| {
            WAKER.register(cx.waker());

            if self.event_happened_and_reset(Event::End) {
                defmt::trace!("RX done poll");
                self.disable_interrupt(Event::End);

                Poll::Ready(())
            } else {
                defmt::trace!("RX enable IRQ");
                self.enable_interrupt(Event::End);
                defmt::trace!("RX pending poll");
                Poll::Pending
            }
        })
        .await;

        dma_end_fence();
        dropper.defuse();

        let timestamp = RadioTimestamps::address_timestamp();
        let rssi = self.radio.rssisample.read().rssisample().bits() as i8;

        defmt::debug!(
            "RX complete, address received at {}, rssi = -{} dBm",
            timestamp,
            rssi
        );

        let crc = self.radio.rxcrc.read().rxcrc().bits() as u16;
        if self.radio.crcstatus.read().crcstatus().bit_is_set() {
            defmt::trace!("RX CRC OK");
            Ok((Timestamp(timestamp), Rssi(-rssi)))
        } else {
            Err(crc)
        }
    }

    unsafe fn start_recv(&mut self, packet: &mut Packet) {
        // NOTE we do NOT check the address of `packet` because the mutable reference ensures it's
        // allocated in RAM

        // clear related events
        self.radio.events_phyend.reset();
        self.radio.events_end.reset();
        self.radio.events_ready.reset();
        self.radio.events_address.reset();

        self.put_in_rx_mode();
        defmt::trace!("Into RX mode");

        // NOTE(unsafe) DMA transfer has not yet started
        // set up RX buffer
        self.radio
            .packetptr
            .write(|w| w.packetptr().bits(packet.buffer.as_mut_ptr() as u32));

        // start transfer
        dma_start_fence();
        self.radio.tasks_start.write(|w| w.tasks_start().set_bit());
        defmt::trace!("Start receiving");
    }

    fn cancel_recv() {
        let radio: pac::RADIO = unsafe { core::mem::transmute(()) };
        radio.tasks_stop.write(|w| w.tasks_stop().set_bit());
        while radio.state.read().state().variant().unwrap() != STATE_A::RX_IDLE {}
        // DMA transfer may have been in progress so synchronize with its memory operations
        dma_end_fence();
    }

    /// Sends the given `packet`
    ///
    /// This is utility method that *consecutively* calls the `try_send` method until it succeeds.
    /// Note that this approach is *not* IEEE spec compliant -- there must be delay between failed
    /// CCA attempts to be spec compliant
    ///
    /// NOTE this method will *not* modify the `packet` argument. The mutable reference is used to
    /// ensure the `packet` buffer is allocated in RAM, which is required by the RADIO peripheral
    // NOTE we do NOT check the address of `packet` because the mutable reference ensures it's
    // allocated in RAM
    pub async fn send(&mut self, packet: &mut Packet) -> Timestamp {
        // enable radio to perform cca
        self.put_in_rx_mode();
        defmt::trace!("In RX mode to find CCA");

        // clear related events
        self.radio.events_phyend.reset();
        self.radio.events_end.reset();
        self.radio.events_ready.reset();

        // immediately start transmission if the channel is idle
        self.radio.shorts.modify(|_, w| {
            w.ccaidle_txen()
                .set_bit()
                .txready_start()
                .set_bit()
                .end_disable()
                .set_bit()
        });

        // the DMA transfer will start at some point after the following write operation so
        // we place the compiler fence here
        dma_start_fence();
        // NOTE(unsafe) DMA transfer has not yet started
        unsafe {
            self.radio
                .packetptr
                .write(|w| w.packetptr().bits(packet.buffer.as_ptr() as u32));
        }

        // start CCA (+ sending if channel is clear)
        self.radio
            .tasks_ccastart
            .write(|w| w.tasks_ccastart().set_bit());

        defmt::trace!("Search for CCA...");

        core::future::poll_fn(|cx| {
            WAKER.register(cx.waker());

            if self.event_happened_and_reset(Event::PhyEnd) {
                self.disable_interrupt(Event::PhyEnd);
                self.disable_interrupt(Event::CcaBusy);

                return Poll::Ready(());
            } else if self.event_happened_and_reset(Event::CcaBusy) {
                // Try CCA again
                self.radio
                    .tasks_ccastart
                    .write(|w| w.tasks_ccastart().set_bit());
                defmt::trace!("Collision, CCA again...");
            }

            self.enable_interrupt(Event::PhyEnd);
            self.enable_interrupt(Event::CcaBusy);

            Poll::Pending
        })
        .await;

        let timestamp = RadioTimestamps::address_timestamp();

        defmt::debug!("TX complete, address sent at: {}", timestamp);

        self.radio.shorts.reset();

        Timestamp(timestamp)
    }

    /// Sends the specified `packet` without first performing CCA
    ///
    /// Acknowledgment packets must be sent using this method
    ///
    /// NOTE this method will *not* modify the `packet` argument. The mutable reference is used to
    /// ensure the `packet` buffer is allocated in RAM, which is required by the RADIO peripheral
    // NOTE we do NOT check the address of `packet` because the mutable reference ensures it's
    // allocated in RAM
    pub async fn send_no_cca(&mut self, packet: &mut Packet) -> Timestamp {
        self.put_in_tx_mode();

        // clear related events
        self.radio.events_phyend.reset();
        self.radio.events_end.reset();

        // NOTE(unsafe) DMA transfer has not yet started
        unsafe {
            self.radio
                .packetptr
                .write(|w| w.packetptr().bits(packet.buffer.as_ptr() as u32));
        }

        // configure radio to disable transmitter once packet is sent
        self.radio.shorts.modify(|_, w| w.end_disable().set_bit());

        // start DMA transfer
        dma_start_fence();
        self.radio.tasks_start.write(|w| w.tasks_start().set_bit());

        core::future::poll_fn(|cx| {
            WAKER.register(cx.waker());

            if self.event_happened_and_reset(Event::PhyEnd) {
                self.disable_interrupt(Event::PhyEnd);
                Poll::Ready(())
            } else {
                self.enable_interrupt(Event::PhyEnd);
                Poll::Pending
            }
        })
        .await;

        let timestamp = RadioTimestamps::address_timestamp();

        self.radio.shorts.reset();

        Timestamp(timestamp)
    }

    /// Moves the radio from any state to the DISABLED state
    fn disable(&mut self) {
        // See figure 110 in nRF52840-PS
        loop {
            match self.radio.state.read().state().variant().unwrap() {
                STATE_A::DISABLED => return,

                STATE_A::RX_RU | STATE_A::RX_IDLE | STATE_A::TX_RU | STATE_A::TX_IDLE => {
                    self.radio
                        .tasks_disable
                        .write(|w| w.tasks_disable().set_bit());

                    self.wait_for_state_a(STATE_A::DISABLED);
                    return;
                }

                // ramping down
                STATE_A::RX_DISABLE | STATE_A::TX_DISABLE => {
                    self.wait_for_state_a(STATE_A::DISABLED);
                    return;
                }

                // cancel ongoing transfer or ongoing CCA
                STATE_A::RX => {
                    self.radio
                        .tasks_ccastop
                        .write(|w| w.tasks_ccastop().set_bit());
                    self.radio.tasks_stop.write(|w| w.tasks_stop().set_bit());
                    self.wait_for_state_a(STATE_A::RX_IDLE);
                }
                STATE_A::TX => {
                    self.radio.tasks_stop.write(|w| w.tasks_stop().set_bit());
                    self.wait_for_state_a(STATE_A::TX_IDLE);
                }
            }
        }
    }

    /// Moves the radio to the RXIDLE state
    fn put_in_rx_mode(&mut self) {
        let state = self.state();

        let (disable, enable) = match state {
            State::Disabled => (false, true),
            State::RxIdle => (false, self.needs_enable),
            // NOTE to avoid errata 204 (see rev1 v1.4) we do TXIDLE -> DISABLED -> RXIDLE
            State::TxIdle => (true, true),
        };

        self.radio.rxaddresses.write(|w| unsafe { w.bits(0xff) });
        self.radio.shorts.modify(|_, w| {
            w.address_rssistart()
                .enabled()
                .disabled_rssistop()
                .enabled()
        });

        if disable {
            self.radio
                .tasks_disable
                .write(|w| w.tasks_disable().set_bit());
            self.wait_for_state_a(STATE_A::DISABLED);
        }

        if enable {
            self.needs_enable = false;
            self.radio.tasks_rxen.write(|w| w.tasks_rxen().set_bit());
            self.wait_for_state_a(STATE_A::RX_IDLE);
        }
    }

    /// Moves the radio to the TXIDLE state
    fn put_in_tx_mode(&mut self) {
        let state = self.state();

        self.radio
            .txaddress
            .write(|w| unsafe { w.txaddress().bits(0) });

        if state != State::TxIdle || self.needs_enable {
            self.needs_enable = false;
            self.radio.tasks_txen.write(|w| w.tasks_txen().set_bit());
            self.wait_for_state_a(STATE_A::TX_IDLE);
        }
    }

    fn state(&self) -> State {
        match self.radio.state.read().state().variant().unwrap() {
            // final states
            STATE_A::DISABLED => State::Disabled,
            STATE_A::TX_IDLE => State::TxIdle,
            STATE_A::RX_IDLE => State::RxIdle,

            // transitory states
            STATE_A::TX_DISABLE => {
                self.wait_for_state_a(STATE_A::DISABLED);
                State::Disabled
            }

            _ => unreachable!(),
        }
    }

    /// Enable interrupt.
    fn enable_interrupt(&self, event: Event) {
        match event {
            Event::End => {
                self.radio.intenset.write(|w| w.end().set_bit());
            }
            Event::PhyEnd => {
                self.radio.intenset.write(|w| w.phyend().set_bit());
            }
            Event::CcaBusy => {
                self.radio.intenset.write(|w| w.ccabusy().set_bit());
            }
        }
    }

    /// Disable interrupt.
    fn disable_interrupt(&self, event: Event) {
        match event {
            Event::End => {
                self.radio.intenclr.write(|w| w.end().set_bit());
            }
            Event::PhyEnd => {
                self.radio.intenclr.write(|w| w.phyend().set_bit());
            }
            Event::CcaBusy => {
                self.radio.intenclr.write(|w| w.phyend().set_bit());
            }
        }
    }

    /// Return true if event has happened.
    fn event_happened_and_reset(&self, event: Event) -> bool {
        match event {
            Event::End => {
                if self.radio.events_end.read().events_end().bit_is_set() {
                    self.radio.events_end.reset();
                    return true;
                }
            }
            Event::PhyEnd => {
                if self.radio.events_phyend.read().events_phyend().bit_is_set() {
                    self.radio.events_phyend.reset();
                    return true;
                }
            }
            Event::CcaBusy => {
                if self
                    .radio
                    .events_ccabusy
                    .read()
                    .events_ccabusy()
                    .bit_is_set()
                {
                    self.radio.events_ccabusy.reset();
                    return true;
                }
            }
        }

        false
    }

    /// Waits until the radio state matches the given `state`
    fn wait_for_state_a(&self, state: STATE_A) {
        while self.radio.state.read().state().variant().unwrap() != state {}
    }
}

/// Error
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Error {
    /// Incorrect CRC
    Crc(u16),
    /// Timeout
    Timeout,
}

/// Driver state
///
/// After, or at the start of, any method call the RADIO will be in one of these states
// This is a subset of the STATE_A enum
#[derive(Copy, Clone, PartialEq)]
enum State {
    Disabled,
    RxIdle,
    TxIdle,
}

/// NOTE must be followed by a volatile write operation
fn dma_start_fence() {
    atomic::compiler_fence(Ordering::Release);
}

/// NOTE must be preceded by a volatile read operation
fn dma_end_fence() {
    atomic::compiler_fence(Ordering::Acquire);
}

enum Event {
    End,
    PhyEnd,
    CcaBusy,
}

/// An IEEE 802.15.4 packet
///
/// This `Packet` is a PHY layer packet. It's made up of the physical header (PHR) and the PSDU
/// (PHY service data unit). The PSDU of this `Packet` will always include the MAC level CRC, AKA
/// the FCS (Frame Control Sequence) -- the CRC is fully computed in hardware and automatically
/// appended on transmission and verified on reception.
///
/// The API lets users modify the usable part (not the CRC) of the PSDU via the `deref` and
/// `copy_from_slice` methods. These methods will automatically update the PHR.
///
/// See figure 119 in the Product Specification of the nRF52840 for more details
pub struct Packet {
    buffer: [u8; Self::SIZE],
}

// See figure 124 in nRF52840-PS
impl Packet {
    // for indexing purposes
    const PHY_HDR: usize = 0;
    const DATA: RangeFrom<usize> = 1..;

    /// Maximum amount of usable payload (CRC excluded) a single packet can contain, in bytes
    pub const CAPACITY: u8 = 125;
    const CRC: u8 = 2; // size of the CRC, which is *never* copied to / from RAM
    const MAX_PSDU_LEN: u8 = Self::CAPACITY + Self::CRC;
    const SIZE: usize = 1 /* PHR */ + Self::MAX_PSDU_LEN as usize;

    /// Returns an empty packet (length = 0)
    pub fn new() -> Self {
        let mut packet = Self {
            buffer: [0; Self::SIZE],
        };
        packet.set_len(0);
        packet
    }

    /// Fills the packet payload with given `src` data
    ///
    /// # Panics
    ///
    /// This function panics if `src` is larger than `Self::CAPACITY`
    pub fn copy_from_slice(&mut self, src: &[u8]) {
        assert!(src.len() <= Self::CAPACITY as usize);
        let len = src.len() as u8;
        self.buffer[Self::DATA][..len as usize].copy_from_slice(&src[..len.into()]);
        self.set_len(len);
    }

    /// Returns the size of this packet's payload
    pub fn len(&self) -> u8 {
        self.buffer[Self::PHY_HDR] - Self::CRC
    }

    /// Changes the size of the packet's payload
    ///
    /// # Panics
    ///
    /// This function panics if `len` is larger than `Self::CAPACITY`
    pub fn set_len(&mut self, len: u8) {
        assert!(len <= Self::CAPACITY);
        self.buffer[Self::PHY_HDR] = len + Self::CRC;
    }
}

impl ops::Deref for Packet {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.buffer[Self::DATA][..self.len() as usize]
    }
}

impl ops::DerefMut for Packet {
    fn deref_mut(&mut self) -> &mut [u8] {
        let len = self.len();
        &mut self.buffer[Self::DATA][..len as usize]
    }
}
