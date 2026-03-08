#![no_std]
#![no_main] #![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

extern crate alloc;
use alloc::vec::Vec;
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::peripherals::GPIO2;
use esp_hal::{
    clock::CpuClock,
    gpio::{
        DriveMode,
        Flex,
        OutputConfig,
        Pull,
        RtcPin,
        RtcPinWithResistors,
        rtc_io::LowPowerOutput,
    },
    load_lp_code,
    main,
    time::Instant,
    ulp_core::{UlpCore, UlpCoreWakeupSource},
};
use log::info;
use hstats::Hstats;
use num_traits::real::Real;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

// Affects how fast the ULP code is executed, and how fast the rainbow changes as a result.
// 530 cycles is about 1Hz.
const ULP_SLEEP_CYCLES: u32 = 53;
const ULP_CYCLES_PER_SECOND : u32 = 530;
const HISTOGRAM_BARS : usize = 30;

#[inline]
fn init_psram_heap(psram: esp_hal::peripherals::PSRAM) {
    // new as of esp-hal v23
    let (start, size) = esp_hal::psram::psram_raw_parts(&psram);
    unsafe {
        esp_alloc::HEAP.add_region(esp_alloc::HeapRegion::new(start, size, esp_alloc::MemoryCapability::External.into()));
    }
}

fn pct_diff(a : f32, b : f32) -> f32 {
    return (a - b).abs() / b.abs()
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_println::logger::init_logger_from_env();

    // Setup a heap allocator of a certain size, which allows for dynamic memory allocation.
    // https://docs.rs/esp-alloc/latest/esp_alloc/#using-this-as-your-global-allocator
    // NOTE: This is still required in esp-hal v23 (no changes).
    esp_alloc::heap_allocator!(size: 72 * 1024);
    // ESP32S3 MINI has 2MB of PSRAM.
    init_psram_heap(peripherals.PSRAM);

    {
        // REQUIRED FOR LEIGHLEIGHLEIGH's CUSTOM DEVBOARD ONLY
        // Turn the power on, and keep it on during sleep using pad hold.
        let mut io_reg_en = peripherals.GPIO2;
        let mut reg_enable = Flex::new(io_reg_en.reborrow());
        reg_enable.apply_output_config(
            &OutputConfig::default()
                .with_drive_mode(DriveMode::OpenDrain)
                .with_pull(Pull::Up),
        );
        reg_enable.set_high();
        <GPIO2 as RtcPin>::rtcio_pad_hold(&io_reg_en, true);
        <GPIO2 as RtcPinWithResistors>::rtcio_pullup(&io_reg_en, true);
    }

    // Pointer to the shared counter variable in memory
    let counter_ptr = (0x5000_1000) as *mut u32;

    // Setup the UlpCore, which will stop it.
    let mut ulp_core = UlpCore::new(peripherals.ULP_RISCV_CORE).with_sleep_cycles(ULP_SLEEP_CYCLES);
    // Load the application from the other crate (build that crate first)
    let ulp_core_code = load_lp_code!("../ulp-rainbow/ulp-rainbow");
    // Reset the counter to 0
    unsafe {
        counter_ptr.write_volatile(0);
    }

    // Using I2C0 and I2C1 for the SCL/SDA pins.
    // let ulp_arg_pin0 = LowPowerOutputOpenDrain::new(peripherals.GPIO0);
    // let ulp_arg_pin1 = LowPowerOutputOpenDrain::new(peripherals.GPIO1);
    // WS2812B data line
    let ulp_arg_gpio18 = LowPowerOutput::new(peripherals.GPIO18);

    ulp_core_code.run(&mut ulp_core, UlpCoreWakeupSource::HpCpu, ulp_arg_gpio18);

    // In a loop, try to measure how fast the counter is updating.
    // This is not a functional part of this demo, it's just something interesting for the HP-core
    // to do.
    let mut last_print_time = Instant::now(); // Print the average rate every second
    let mut last_change_time = Instant::now();
    let mut last_counter = unsafe { counter_ptr.read_volatile() };

    // Calculate lower/upper bounds
    let expected_mean = 1000.0 * (ULP_SLEEP_CYCLES as f32) / (ULP_CYCLES_PER_SECOND as f32) + 0.7;
    let mean_lo = expected_mean * 0.5;
    let mean_hi = expected_mean * 2.0;
    let mut stathist = Hstats::new(mean_lo, mean_hi, HISTOGRAM_BARS);

    loop {
        let new_count = unsafe { counter_ptr.read_volatile() };
        let new_time = Instant::now();

        if new_count != last_counter {
            let dc = new_count - last_counter;
            let dt = new_time - last_change_time;
            let dtmicros = dt.as_micros();
            let count_period = dtmicros / (dc as u64);

            // Add to hist
            let sample = (count_period as f32) / 1000.0;
            stathist.add(sample);

            if last_print_time.elapsed().as_millis() >= 500 {
                // Query statistics
                let hist = stathist.clone();
                // info!("count: {}, mean: {:.2}, std_dev: {:.2}", hist.count(), hist.mean(), hist.std_dev());
                // info!("min: {:.2}, max: {:.2}", hist.min(), hist.max());
                // Print the histogram
                info!("{}", hist.clone().with_precision(5));
                let percentiles : Vec<(f32,f32)> = hist.bins_at_centiles(&[50, 90]).iter().map(|(lower,_upper,_n)| (lower.clone(),_upper.clone())).collect();
                info!("50%: ({}, {})", percentiles[0].0, percentiles[0].1);
                info!("90%: ({}, {})", percentiles[1].0, percentiles[1].1);

                last_print_time = Instant::now();
            }

            // Account for any delays due to histogram printing
            last_counter = unsafe { counter_ptr.read_volatile() };
            last_change_time = Instant::now();

            //last_counter = new_count;
            //last_change_time = new_time;
        }
    }
}
