#![allow(unused)]

use smart_leds::RGB8;

// Const-time conversion utilities

// Public runtime utils

pub fn apply_brightness(a: RGB8, brightness: u16) -> RGB8 {
    RGB8::new(
        scale_brightness(a.r as u16, brightness),
        scale_brightness(a.g as u16, brightness),
        scale_brightness(a.b as u16, brightness),
    )
}

pub const fn apply_gamma(a: RGB8) -> RGB8 {
    // Casts up to u16, calculates gamma, down casts to u8
    RGB8::new(scale_gamma(a.r), scale_gamma(a.g), scale_gamma(a.b))
}

// Private runtime utils

fn scale_brightness(v: u16, b: u16) -> u8 {
    (v * (b + 1) / 256) as u8
}


const fn scale_gamma(val: u8) -> u8 {
    // Fastest option - just a square
    // ((value * value) >> 16) as u16

    // Second fastest, a cube
    // let value : u16 = val as u16;
    // let gam = (((value * value) >> 8) * value ) >> 8;

    // More accurate option
    let value: u32 = val as u32;
    let a: u32 = (value * value) >> 8;
    let b: u32 = (a * a) >> 8;
    let gam = (a + b) / 2; // Get average of the two
    gam as u8
}