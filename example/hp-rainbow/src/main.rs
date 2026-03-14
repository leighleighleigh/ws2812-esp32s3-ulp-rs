#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_backtrace as _;
use esp_hal::delay::Delay;
// For power pin
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

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

// Affects how fast the ULP code is executed, and how fast the rainbow changes as a result. 530
// cycles is about 1Hz. const ULP_SLEEP_CYCLES : u32 = 265;
const ULP_SLEEP_CYCLES: u32 = 10;

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

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
    let counter_ptr = (0x5000_1800) as *mut u32;

    // Setup the UlpCore, which will stop it.
    let mut ulp_core = UlpCore::new(peripherals.ULP_RISCV_CORE);
    // Load the application from the other crate (build that crate first)
    let ulp_core_code = load_lp_code!("../ulp-rainbow/ulp-rainbow");
    // Reset the counter to 0
    unsafe {
        counter_ptr.write_volatile(0);
    }

    // WS2812B data line
    let ulp_arg_gpio18 = LowPowerOutput::new(peripherals.GPIO18);
    ulp_core_code.run(&mut ulp_core, UlpCoreWakeupSource::HpCpu, ulp_arg_gpio18);

    // In a loop, try to measure how fast the counter is updating.
    // This is not a functional part of this demo,
    // it's just something interesting for the HP-core to do.
    let mut dly = Delay::new();
    let mut last_print_time = Instant::now(); // Print the average rate every second
    let mut last_change_time = Instant::now();
    let mut last_counter = unsafe { counter_ptr.read_volatile() };
    let mut single_count_samples: u64 = 0;
    let mut single_count_period: u64 = 0;

    loop {
        let new_count = unsafe { counter_ptr.read_volatile() };
        let new_time = Instant::now();

        dly.delay_millis(10);

        if new_count != last_counter {
            let dc = new_count - last_counter;
            let dt = new_time - last_change_time;
            // Calculate micros
            let dtmicros = dt.as_micros();
            // calculate micros per count
            let count_period = dtmicros / (dc as u64);
            single_count_samples += 1;
            single_count_period += count_period;
            last_counter = new_count;
            last_change_time = new_time;

            if last_print_time.elapsed().as_millis() >= 1000 {
                let avg_period = single_count_period / single_count_samples;
                let avg_rate = 1000000.0 / (avg_period as f64);
                info!(
                    "counter {}, samples {}, mean_count_rate {:.3} Hz",
                    new_count, single_count_samples, avg_rate
                );
                last_print_time = Instant::now();
            }
        }
    }
}
