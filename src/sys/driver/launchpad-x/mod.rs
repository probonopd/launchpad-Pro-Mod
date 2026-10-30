// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2025-2026 Anthony Hofmeister
// Copyright (C) 2026 ZephyrCodesStuff

pub mod grid;
pub mod inputs;
pub mod led_scan;
pub mod leds;
pub mod runtime;
pub mod usb;

use crate::sys::driver::common::storage::ExtFlash;

use embassy_executor::Spawner;
use embassy_stm32::peripherals;
use embassy_stm32::rcc::*;
use embassy_stm32::Peri;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Ticker};
use crate::app::sequencer::SequencerApp;
use crate::app::{AftertouchEvent, AppHost, AppId, SurfaceEvent};
use crate::sys::driver;
use crate::sys::settings;
use static_cell::StaticCell;

type SharedAppHost = Mutex<CriticalSectionRawMutex, AppHost>;

static GRID: StaticCell<grid::Grid> = StaticCell::new();
static RUNTIME_DRIVER: StaticCell<runtime::RuntimeDriver> = StaticCell::new();
static APP_HOST: StaticCell<SharedAppHost> = StaticCell::new();
static SEQ95: StaticCell<SequencerApp> = StaticCell::new();

// The big structures are built in functions of their own, built in place in their
// statics. This keeps the temporaries of the construction out of the stack frame
// of the main task, which is alive every time the executor polls it: with the
// construction inside it, that frame (37 KB) was larger than the free RAM once the
// sequencer's project buffer was added, and the firmware crashed before USB came up.
#[inline(never)]
fn init_grid() -> &'static mut grid::Grid {
    GRID.init_with(grid::Grid::new)
}

#[inline(never)]
fn init_runtime_driver(
    grid: &mut grid::Grid,
    spi2: Peri<'static, peripherals::SPI2>,
    pb13: Peri<'static, peripherals::PB13>,
    pb15: Peri<'static, peripherals::PB15>,
    pb14: Peri<'static, peripherals::PB14>,
    pb12: Peri<'static, peripherals::PB12>,
) -> &'static mut runtime::RuntimeDriver {
    RUNTIME_DRIVER.init_with(|| runtime::RuntimeDriver::new(grid, ExtFlash::new(spi2, pb13, pb15, pb14, pb12)))
}

// The sequencer is the biggest app and has its own function, so that its temporaries
// and those of the app host are never on the stack together.
#[inline(never)]
fn init_sequencer() -> &'static mut SequencerApp {
    SEQ95.init_with(SequencerApp::new)
}

#[inline(never)]
fn init_app_host(seq95: &'static mut SequencerApp) -> &'static mut SharedAppHost {
    APP_HOST.init_with(|| Mutex::new(AppHost::new_with_sequencer(AppId::Boot, seq95)))
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = embassy_stm32::Config::default();

    config.rcc.hse = Some(Hse {
        freq: embassy_stm32::time::Hertz(24_000_000),
        mode: HseMode::Oscillator,
    });

    config.rcc.pll_src = PllSource::HSE;

    config.rcc.pll = Some(Pll {
        prediv: PllPreDiv::DIV12,
        mul: PllMul::MUL168,
        divp: Some(PllPDiv::DIV4),
        divq: Some(PllQDiv::DIV7),
        divr: None,
    });

    config.rcc.ahb_pre = AHBPrescaler::DIV1;
    config.rcc.apb1_pre = APBPrescaler::DIV2;
    config.rcc.apb2_pre = APBPrescaler::DIV1;
    config.rcc.sys = Sysclk::PLL1_P;
    config.rcc.mux.clk48sel = mux::Clk48sel::PLL1_Q;

    let p = embassy_stm32::init(config);

    usb::init_event_queues();
    usb::init();

    let grid = init_grid();
    let runtime_driver = init_runtime_driver(grid, p.SPI2, p.PB13, p.PB15, p.PB14, p.PB12);
    driver::install(runtime_driver);
    settings::load();
    crate::sys::prefs::load();

    let sequencer = init_sequencer();
    let app_host = init_app_host(sequencer);
    app_host.lock().await.init();

    led_scan::start(grid);

    let mut ticker = Ticker::every(Duration::from_millis(1));

    loop {
        ticker.next().await;

        let mut app_host_guard = app_host.lock().await;

        while let Some(event) = usb::dequeue_midi_event() {
            app_host_guard.route_midi_event(event);
        }

        app_host_guard.route_tick_event();

        grid.process_inputs();
        while let Some(event) = grid.poll_event() {
            match event {
                inputs::GridEvent::Press { note, value } => {
                    app_host_guard.route_surface_event(SurfaceEvent {
                        pressed: true,
                        index: note,
                        value,
                    });
                }
                inputs::GridEvent::Release { note } => {
                    app_host_guard.route_surface_event(SurfaceEvent {
                        pressed: false,
                        index: note,
                        value: 0,
                    });
                }
                inputs::GridEvent::Aftertouch { note, value } => {
                    app_host_guard.route_aftertouch_event(AftertouchEvent { index: note, value });
                }
            }
        }

        while let Some(message) = usb::dequeue_sysex_message() {
            app_host_guard.receive_sysex(message.port, &message.data[..message.len]);
        }
    }
}
