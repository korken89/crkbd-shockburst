use crate::app::*;
use rtic_monotonics::nrf::timer::*;

pub async fn task(cx: task::Context<'_>) -> ! {
    let led = cx.local.led;

    loop {
        led.set_high();
        Timer0::delay(500.millis()).await;
        led.set_low();
        Timer0::delay(500.millis()).await;
    }
}
