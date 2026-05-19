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
use esp_hal::gpio::{ Output, OutputConfig, Level, Pull, Input, InputConfig };
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


enum TrafficLightState {
    Green { pedestrian_waiting: bool },
    Amber { pedestrian_waiting: bool },
    Red { pedestrian_waiting: bool },
}

enum TrafficLightSequence {
    Green = 0,
    Amber = 1,
    Red = 2,
}

static BTN_EVT: Channel<CriticalSectionRawMutex, ButtonEvent, 4> = Channel::new();

/*
 * Reference to FSM explosion example, due to boolean flag.
 * See EFS, FSM and Cartisianal product
 *
 * initial state: Red(false)
 * State        > Event         > Next State    > Period
 * Green(false) > Tick          > Amber(false)  > 1s
 * Green(false) > IsPressed     > Green(true)   > 2s
 * Green(true)  > Tick          > Amber(false)  > 1s 
 * Green(true)  > IsPressed     > Green(true)   > 2s
 *
 * Amber(false) > Tick          > Red(false)    > 3s
 * Amber(false) > IsPressed     > Amber(true)   > 1s
 * Amber(true)  > Tick          > Red(true)     > 3s 
 * Amber(true)  > IsPressed     > Amber(true)   > 1s
 *
 * Red(false)   > Tick          > Green(false)  > 3s 
 * Red(false)   > IsPressed     > Red(false)    > 3s
 * Red(true)    > Tick          > Green(true)   > 2s
 * Red(true)    > IsPressed     > Red(true)     > 3s 
 * */
 
struct TrafficLightPeriod {
    green_waiting: u64,
    green: u64,
    amber: u64,
    red: u64
}

impl TrafficLightPeriod {
    fn new() -> Self{
        Self {
            green_waiting : 2000,
            green : 3000,
            amber : 1000,
            red : 3000,
        }
    }
}


fn traffic_light_transition(state: TrafficLightState, event: ButtonEvent) -> (TrafficLightState, u64){
    let period = TrafficLightPeriod::new();

        match (state, event) {
            (TrafficLightState::Green { pedestrian_waiting: false }, ButtonEvent::Tick) => {
                (TrafficLightState::Amber { pedestrian_waiting: false }, period.amber)
            }
            (TrafficLightState::Green { pedestrian_waiting: false }, ButtonEvent::IsPressed) => {
                (TrafficLightState::Green { pedestrian_waiting: true }, period.green_waiting)
            }
            (TrafficLightState::Green { pedestrian_waiting: true }, ButtonEvent::Tick) => {
                (TrafficLightState::Amber { pedestrian_waiting: false }, period.amber)
            }
            (TrafficLightState::Green { pedestrian_waiting: true }, ButtonEvent::IsPressed) => {
                (TrafficLightState::Green { pedestrian_waiting: true }, period.green_waiting)
            }

            (TrafficLightState::Amber { pedestrian_waiting: false }, ButtonEvent::Tick) => {
                (TrafficLightState::Red { pedestrian_waiting: false}, period.red)
            }
            (TrafficLightState::Amber { pedestrian_waiting: false }, ButtonEvent::IsPressed) => {
                (TrafficLightState::Amber { pedestrian_waiting: false }, period.amber)
            }
            (TrafficLightState::Amber { pedestrian_waiting: true }, ButtonEvent::Tick) => {
                (TrafficLightState::Red { pedestrian_waiting: true }, period.red)
            }
            (TrafficLightState::Amber { pedestrian_waiting: true }, ButtonEvent::IsPressed) => {
                (TrafficLightState::Amber { pedestrian_waiting: true }, period.amber)
            }

            (TrafficLightState::Red { pedestrian_waiting: false }, ButtonEvent::Tick) => {
                (TrafficLightState::Green { pedestrian_waiting: false }, period.green)
            }
            (TrafficLightState::Red { pedestrian_waiting: false }, ButtonEvent::IsPressed) => {
                (TrafficLightState::Red { pedestrian_waiting: true }, period.red)
            }
            (TrafficLightState::Red { pedestrian_waiting: true }, ButtonEvent::Tick) => {
                (TrafficLightState::Green { pedestrian_waiting: false }, period.green_waiting)
            }
            (TrafficLightState::Red { pedestrian_waiting: true }, ButtonEvent::IsPressed) => {
                (TrafficLightState::Red { pedestrian_waiting: true }, period.red)
            }
        } 
}

#[embassy_executor::task]
async fn button_event(mut led_arr: [Output<'static>; 3], mut ped_led: Output<'static>){
    let mut traffic_light_state = TrafficLightState::Red { pedestrian_waiting: false };
    let mut duration = 3000;

    loop {
        let button_event_task = BTN_EVT.receive();
        match select(button_event_task, Timer::after_millis(duration)).await {
            Either::First(button_event_task) => {
                (traffic_light_state, duration) = traffic_light_transition(traffic_light_state, button_event_task);
                ped_led.set_high();
            }
            Either::Second(()) => {
                match traffic_light_state {
                    TrafficLightState::Green { .. } => {
                        led_arr[0].set_low();
                        led_arr[1].set_high();
                        ped_led.set_low();
                    }
                    TrafficLightState::Amber { .. } => {
                        led_arr[1].set_low();
                        led_arr[2].set_high();
                    }
                    TrafficLightState::Red { .. } => {
                        led_arr[2].set_low();
                        led_arr[0].set_high();
                    }
                }        
            }
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

    let output_cfg = OutputConfig::default().with_pull(Pull::Up);
    let green = Output::new(peripherals.GPIO6, Level::Low, output_cfg);
    let amber = Output::new(peripherals.GPIO7, Level::Low, output_cfg);
    let red = Output::new(peripherals.GPIO8, Level::High, output_cfg);
    let ped = Output::new(peripherals.GPIO5, Level::Low, output_cfg);

    // TODO: Spawn some tasks
    let _ = spawner;

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}
