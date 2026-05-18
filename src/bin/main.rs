#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::{Spawner};
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use embassy_sync::channel::{Channel};
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex};
use embassy_futures::select::{select, Either};


#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();


enum ButtonEvent {
    Tick,
    IsPressed
}


enum TrafficLight {
    Red { pedestrian_waiting: bool },
    Amber { pedestrian_waiting: bool },
    Green { pedestrian_waiting: bool },
}

static BTN_EVT: Channel<CriticalSectionRawMutex, ButtonEvent, 4> = Channel::new();

/*
 *
 * State        > Event         > Period
 * Red(false)   > Tick          > 3s 
 * Red(false)   > IsPressed     > 3s
 * Red(true)    > Tick          > 3s
 * Red(true)    > IsPressed     > 3s 
 *
 * Amber(false) > Tick          > 1s
 * Amber(false) > IsPressed     > 1s
 * Amber(true)  > Tick          > 1s 
 * Amber(true)  > IsPressed     > 1s
 *
 * Green(false) > Tick          > 3s
 * Green(false) > IsPressed     > 2s
 * Green(true)  > Tick          > 2s 
 * Green(true)  > IsPressed     > 2s
 * */
 

#[embassy_executor::task]
async fn button_event(){
    let state = TrafficLight::Red;

    loop {
        match select(BTN_EVT.receive(),Timer::after_millis(x)) {

        } 
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32c3 -o unstable-hal -o embassy -o wokwi -o neovim -o vscode

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    // TODO: Spawn some tasks
    let _ = spawner;

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}
