//! Increments a 32 bit counter value at a known point in memory.
//! Uses the counter value to calculate an RGB Hue, and sets a single WS2812B led to this hue.
//!
//! When using the ESP32-S2 or ESP32-S3's ULP core, this address in memory is
//! `0x5000_1000` (but is `0x1000`` from the ULP's point of view!).

#![no_std]
#![no_main]

use esp_lp_hal::{delay::Delay, gpio::Output, prelude::*};
// use embedded_hal::delay::DelayNs;
use panic_halt as _;
use smart_leds::{
    SmartLedsWrite,
    hsv::{Hsv, hsv2rgb},
};
use ws2812_esp32s3_ulp::Ws2812;

mod colours;
use colours::apply_brightness;

#[cfg(any(esp32s2, esp32s3))]
const ADDRESS: u32 = 0x1000;

// Note that, due to how the ws2812b library is CURRENTLY WRITTEN,
// the pin number needs to be a const-time thing.
// THIS MAY CHANGE IN FUTURE. It was originally done to ensure optimal assembly generation.
const PIN_NUMBER: u8 = 18;

#[entry]
fn main(gpio18_led: Output<18>) {
    // Read counter
    let counter_ptr = ADDRESS as *mut u32;
    let mut i: u32 = unsafe { counter_ptr.read_volatile() };

    // Calculate Hue, update WS2812B led.
    let hsv = Hsv {
        hue: (i & 0xFF) as u8,
        sat: 255,
        val: 64,
    };

    // // This is still needed, it prevents WS2812 glitches/flashes on boot up.
    // // EDIT: Now handled in Ws2812::new()
    // let _ = gpio18_led.set_low();
    // Timer::after(Duration::from_millis(10));

    let ws_clk = Delay {};
    let mut ws = Ws2812::<PIN_NUMBER, Delay>::new(ws_clk, gpio18_led);
    let b: u16 = 32;
    let rgb = apply_brightness(hsv2rgb(hsv), b);

    // critical_section::with(|cs| {
    let _ = ws.write([rgb]);
    // });

    // Increment counter before sleeping
    i = i.wrapping_add(1u32);
    unsafe {
        counter_ptr.write_volatile(i);
    }
}
