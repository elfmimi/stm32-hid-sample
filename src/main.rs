//! USB HID Keyboard example using polling in a busy loop.
// based on https://github.com/atsamd-rs/atsamd/blob/master/boards/itsybitsy_m0/examples/twitching_usb_mouse.rs
#![no_std]
#![no_main]

extern crate panic_semihosting;

use cortex_m_rt::entry;
use cortex_m::peripheral::SYST;
use stm32_usbd::UsbBus;
use stm32f0xx_hal::{prelude::*, stm32};
use usb_device::prelude::*;
#[cfg(feature = "mouse")]
use usbd_hid::{hid_class::HIDClass, descriptor::{MouseReport, generator_prelude::*}};
#[cfg(not(feature = "mouse"))]
use usbd_hid::{hid_class::HIDClass, descriptor::{KeyboardReport, generator_prelude::*}};

#[entry]
fn main() -> ! {
    let mut cp = cortex_m::Peripherals::take().unwrap();
    let mut dp = stm32::Peripherals::take().unwrap();

    cp.SYST.set_reload(0xFFFFFF);
    cp.SYST.enable_counter();

    let mut rcc = dp
        .RCC
        .configure()
        .hsi48()
        .enable_crs(dp.CRS)
        .sysclk(48.mhz())
        .pclk(24.mhz())
        .freeze(&mut dp.FLASH);

    // Configure the on-board LED
    let gpiob = dp.GPIOB.split(&mut rcc);
    let mut led = cortex_m::interrupt::free(|cs| {
        gpiob.pb12.into_push_pull_output(cs)
    });
    led.set_low().unwrap(); // Turn on

    let gpioa = dp.GPIOA.split(&mut rcc);

    let usb_dm = gpioa.pa11;
    let usb_dp = gpioa.pa12;

    let usb_bus = UsbBus::new(dp.USB, (usb_dm, usb_dp));

    #[cfg(feature = "mouse")]
    let mut hid = HIDClass::new(&usb_bus, MouseReport::desc(), 60);
    #[cfg(not(feature = "mouse"))]
    let mut hid = HIDClass::new(&usb_bus, KeyboardReport::desc(), 60);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Fake company")
        .product("stm32-hid-sample")
        .serial_number("TEST")
        .device_class(0xEF) // misc
        .build();

    let mut elapsed = 0;
    let mut count = SYST::get_current();
    let mut toggle = false;
    loop {
        let new_count = SYST::get_current();
        if count >= new_count {
            elapsed = elapsed + (count - new_count);
        } else {
            elapsed = elapsed + (count + 0x1000000 - new_count);
        }
        count = new_count;

        usb_dev.poll(&mut [&mut hid]);

        if !toggle {
            if elapsed >= 5 * 1000 * 1000 {
                elapsed -= 5 * 1000 * 1000;
                #[cfg(feature = "mouse")]
                hid.push_input(&MouseReport{x: 4, y: 0, buttons: 0}).unwrap();
                #[cfg(not(feature = "mouse"))]
                {
                    let mut keycodes = [0u8; 6];
                    keycodes[0] = 4; // 'A'
                    hid.push_input(&KeyboardReport{modifier: 0, leds: 0, keycodes}).unwrap();
                }
                toggle = !toggle;
            }
        } else {
            if elapsed >= 1 * 1000 * 1000 {
                elapsed -= 1 * 1000 * 1000;
                #[cfg(feature = "mouse")]
                hid.push_input(&MouseReport{x: -4, y: 0, buttons: 0}).unwrap();
                #[cfg(not(feature = "mouse"))]
                {
                    let keycodes = [0u8; 6];
                    hid.push_input(&KeyboardReport{modifier: 0, leds: 0, keycodes}).unwrap();
                }
                toggle = !toggle;
            }
        }
    }
}
