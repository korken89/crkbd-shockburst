use core::{mem, ptr::NonNull};

use embassy_nrf::{
    config::HfclkSource,
    gpio::{Input, Level, Output, OutputDrive, Pull},
    pac,
    peripherals::{P0_00, P0_03, PPI_CH0, PPI_CH1, PPI_CH2},
    ppi::{Event, Ppi, Task},
};
use rtic_monotonics::{
    nrf::timer::{fugit::TimerInstantU32, Timer0},
    Monotonic,
};

pub mod radio;

pub type DongleLed = Output<'static, P0_00>;
pub type Button = Input<'static, P0_03>;

pub struct DongleBsp {
    pub led: DongleLed,
    pub button: Button,
    pub radio: radio::Radio,
}

#[inline(always)]
pub fn init_dongle(_: cortex_m::Peripherals) -> DongleBsp {
    defmt::info!("BSP init");

    let mut config = embassy_nrf::config::Config::default();
    config.hfclk_source = HfclkSource::ExternalXtal;
    let p = embassy_nrf::init(config);

    // Systick monotonic uses CC 0, 1 and 2. We use 3, 4, 5 for capturing radio events.
    let systick_token = rtic_monotonics::create_nrf_timer0_monotonic_token!();
    Timer0::start(unsafe { core::mem::transmute(()) }, systick_token);

    RadioTimestamps::start(p.PPI_CH0);
    let radio: pac::RADIO = unsafe { core::mem::transmute(()) };
    let radio = radio::Radio::init(radio);

    DongleBsp {
        led: Output::new(p.P0_00, Level::Low, OutputDrive::Standard),
        button: Input::new(p.P0_03, Pull::Up),
        radio,
    }
}

/// Hacky way to timestamp in monotonic time.
#[derive(Copy, Clone, Debug, defmt::Format)]
pub struct RadioTimestamps {
    pub ready: TimerInstantU32<1_000_000>,
    pub address: TimerInstantU32<1_000_000>,
    pub phyend: TimerInstantU32<1_000_000>,
}

impl RadioTimestamps {
    fn start(ppi_ch0: PPI_CH0) {
        let Tim0CaptureTasks { cc3, .. } = tim0_capture_tasks();
        let RadioEvents { address, .. } = radio_events();

        // Make PPI capture radio events to unused CC channels of the monotonic.
        let mut ppi = Ppi::new_one_to_one(ppi_ch0, address, cc3);
        ppi.enable();
        mem::forget(ppi);

        // let mut ppi = Ppi::new_one_to_one(ppi_ch1, address, cc4);
        // ppi.enable();
        // mem::forget(ppi);

        // let mut ppi = Ppi::new_one_to_one(ppi_ch2, phy_end, cc5);
        // ppi.enable();
        // mem::forget(ppi);
    }

    pub fn now() -> <Timer0 as Monotonic>::Instant {
        Timer0::now()
    }

    // /// The Radio's events timestamped to the low 32 bits of the monotonic.
    // /// About once every 4200 seconds this will glitch.
    // pub fn timestamps() -> Self {
    //     RadioTimestamps {
    //         ready: Self::ready_timestamp(),
    //         address: Self::address_timestamp(),
    //         phyend: Self::phy_end_timestamp(),
    //     }
    // }

    // /// The Radio's READY event timestamped to the low 32 bits of the monotonic.
    // /// About once every 4200 seconds this will glitch.
    // pub fn ready_timestamp() -> TimerInstantU32<1_000_000> {
    //     TimerInstantU32::from_ticks(unsafe { &*pac::TIMER0::PTR }.cc[3].read().cc().bits())
    // }

    /// The Radio's ADDRESS event timestamped to the low 32 bits of the monotonic.
    /// About once every 4200 seconds this will glitch.
    pub fn address_timestamp() -> TimerInstantU32<1_000_000> {
        TimerInstantU32::from_ticks(unsafe { &*pac::TIMER0::PTR }.cc[3].read().cc().bits())
    }

    // /// The Radio's PHYEND event timestamped to the low 32 bits of the monotonic.
    // /// About once every 4200 seconds this will glitch.
    // pub fn phy_end_timestamp() -> TimerInstantU32<1_000_000> {
    //     TimerInstantU32::from_ticks(unsafe { &*pac::TIMER0::PTR }.cc[5].read().cc().bits())
    // }
}

pub struct RadioEvents {
    pub ready: Event<'static>,
    pub address: Event<'static>,
    pub phy_end: Event<'static>,
}

fn radio_events() -> RadioEvents {
    let radio = unsafe { &*pac::RADIO::PTR };

    RadioEvents {
        ready: unsafe {
            Event::new_unchecked(NonNull::new_unchecked(
                radio.events_ready.as_ptr() as *const _ as *mut _,
            ))
        },
        address: unsafe {
            Event::new_unchecked(NonNull::new_unchecked(
                radio.events_address.as_ptr() as *const _ as *mut _,
            ))
        },
        phy_end: unsafe {
            Event::new_unchecked(NonNull::new_unchecked(
                radio.events_phyend.as_ptr() as *const _ as *mut _,
            ))
        },
    }
}

pub struct Tim0CaptureTasks {
    pub cc3: Task<'static>,
    pub cc4: Task<'static>,
    pub cc5: Task<'static>,
}

fn tim0_capture_tasks() -> Tim0CaptureTasks {
    let tim = unsafe { &*pac::TIMER0::PTR };

    Tim0CaptureTasks {
        cc3: unsafe {
            Task::new_unchecked(NonNull::new_unchecked(
                tim.tasks_capture[3].as_ptr() as *const _ as *mut _,
            ))
        },
        cc4: unsafe {
            Task::new_unchecked(NonNull::new_unchecked(
                tim.tasks_capture[4].as_ptr() as *const _ as *mut _,
            ))
        },
        cc5: unsafe {
            Task::new_unchecked(NonNull::new_unchecked(
                tim.tasks_capture[5].as_ptr() as *const _ as *mut _,
            ))
        },
    }
}
