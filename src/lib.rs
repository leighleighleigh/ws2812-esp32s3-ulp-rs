#![no_std]
#![no_main]
use core::convert::Into;
use core::iter::IntoIterator;
use core::result::Result;
use embedded_hal::digital::OutputPin;
use embedded_hal::delay::DelayNs;
use smart_leds_trait::{RGB8, SmartLedsWrite};


const INIT_RESET_TIME_USEC : u32 = 256;
const RESET_TIME_USEC : u32 = 64;

pub struct Ws2812<const PIN: u8, TIMER> {
    delay: TIMER,
}

impl<const PIN: u8, TIMER> Ws2812<PIN, TIMER> 
where 
    TIMER: DelayNs
{
    pub fn new<O>(mut delay: TIMER, mut pin : O) -> Ws2812<PIN,TIMER> where O: OutputPin {
        // By providing a pin and delay, we can force-reset the chain to prevent start-up glitches.
        pin.set_low().ok();
        // Force a low-reset.
        delay.delay_us(INIT_RESET_TIME_USEC);
        Self { delay }
    }
}

impl<const PIN: u8, TIMER> SmartLedsWrite for Ws2812<PIN,TIMER>
where
    TIMER: DelayNs
{
    type Error = ();
    type Color = RGB8;

    /// Write all the items of an iterator to a ws2812 strip
    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        // Small delays are added to trigger the reset period (>= 50 usec)
        self.delay.delay_us(RESET_TIME_USEC);

        for item in iterator {
            let item = item.into();
            write_rgb::<PIN>(&item);
        }

        Ok(())
    }
}

#[inline]
fn write_rgb<const PIN: u8>(rgb: &RGB8) {
    // Pack the RGB values into a single u32 variable
    let data: u32 = (rgb.g as u32) << 16 | (rgb.r as u32) << 8 | (rgb.b as u32);
    write_impl::<PIN>(data);
}

#[inline]
#[allow(unused_variables)]
fn write_impl<const PIN: u8>(data : u32) {
    // The implementation below is for the esp32s3 (and possibly s2) ULP RISCV only!!!
    // NOTE: This assembly has been hand-optimised for speed.
    unsafe {
        core::arch::asm!(
        // a1 = gpio register address (0xA400)
        "lui a1, 0xa",        // 0xa000
        "addi a1, a1, 0x400", //0xa400
        // a2 = gpio pin bitmask.
        // NOTE: GPIO0 starts at bit 10, GPIO21 is bit 31,
        //       the first 10 bits are reserved.
        "addi a2, x0, 0x400",
        // shift to get to the pin number we want!
        "slli a2, a2, {pin}", 
        // a3 = mask of the data, starting at bit 23 (MSB).
        // NOTE: This mask will be right-shifted each iteration,
        //       so that we can read all the input bits.
        // 0x80_00_00 == 0b10000000_00000000_00000000
        "lui a3, 0x800", 
        // ------------------------------------------ LABEL 1. |
        //                           Check the exit condition. |
        "1:",
        // If the bitmask in a3 is equal to 0,
        // then we have shifted out all the bits.
        // NOTE: Label 4 is the exit point.
        "beq a3, x0, 4f",
        // ------------------------------------------ LABEL 2. |
        //                                      Read data bit, |
        //                             prepare next bit shift, |
        //                                  transmit data bit. |
        "2:",
        // a5 = data & a3 
        "and a5, a3, {dat}",
        // right shift a3 by 1 bit, for the next iteration.
        "srli a3, a3, 1",
        // IF (a5 == 0) THEN (GOTO LABEL 3, transmit a 0) ELSE (transmit a 1)
        "beq a5, x0, 3f",
        // Transmit a '1' bit.
        // NOTE: 1 bit timing is HIGH 800ns, then LOW 450ns.
        // set pin high
        "sw a2, 0x4(a1)", 
        // delay
        "addi x0, x0, 0", 
        // delay
        "addi x0, x0, 0",
        // ------------------------------------------ LABEL 3. |
        //                        Finish transmitting a 1 bit, |
        //          or transmit a 0 bit if jumped to directly. |
        "3:",
        // NOTE: 0 bit timing is HIGH ~450ns, then LOW ~800ns.
        // set pin high (does nothing if already high)
        "sw a2, 0x4(a1)", 
        // set pin low
        "sw a2, 0x8(a1)", 
        // delay
        "addi x0, x0, 0", 
        // Return to LABEL 1, to continue iterating.
        "jal x0, 1b", 
        // ------------------------------------------ LABEL 3. |
        //                                               Exit. |
        "4:",
        // nop
        "addi x0, x0, 0",
        pin = const PIN,
        dat = in(reg) data,
        );
    }
}
