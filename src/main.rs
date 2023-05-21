#![no_std]
#![no_main]

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use defmt::*;
use defmt_rtt as _;
use panic_probe as _;
use rp_pico as board;

use board::{
    entry,
    hal::{self, pac, prelude::*},
};
use eh1_0_alpha::digital::{InputPin, OutputPin};
use embedded_graphics::{
    mono_font::{self, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Point, Size},
    primitives::Rectangle,
    text::{Baseline, Text},
    Drawable,
};
use fugit::RateExtU32;
use hal::{
    gpio::{FunctionI2C, PinState},
    pll::PLLConfig,
    I2C,
};
use timings::DramTimingConfig;
use ufmt::uwrite;

mod clocks;
mod delay;
mod timings;

const CLOCK: (u32, PLLConfig) = clocks::CLOCK_125;
const NS_PER_CYCLE: u32 = 1_000_000_000 / CLOCK.0;

type Timings = timings::Dram150Ns;

const SN74HCT_DELAY: u32 = 14u32.saturating_sub(NS_PER_CYCLE);
/// Add if longer leads, ringing, etc.
const ADDR_SETTLE: u32 = 0;

#[inline(always)]
fn delay_ns<const NS: u32>() {
    delay::delay_ns::<NS, { CLOCK.0 }>();
}

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let clocks = clocks::init_clocks_and_plls(
        CLOCK.1,
        // External high-speed crystal on the pico board is 12Mhz
        12_000_000u32,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let core = pac::CorePeripherals::take().unwrap();
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    // let voltage rails settle before initializing
    delay.delay_ms(500);

    let sio = hal::Sio::new(pac.SIO);
    let pins = board::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led = pins.led.into_push_pull_output();

    // address pins
    pins.gpio0.into_push_pull_output_in_state(PinState::Low);
    pins.gpio1.into_push_pull_output_in_state(PinState::Low);
    pins.gpio2.into_push_pull_output_in_state(PinState::Low);
    pins.gpio3.into_push_pull_output_in_state(PinState::Low);
    pins.gpio4.into_push_pull_output_in_state(PinState::Low);
    pins.gpio5.into_push_pull_output_in_state(PinState::Low);
    pins.gpio6.into_push_pull_output_in_state(PinState::Low);
    pins.gpio7.into_push_pull_output_in_state(PinState::Low);
    pins.gpio8.into_push_pull_output_in_state(PinState::Low);

    // ~WRT pin
    let we = pins.gpio11.into_push_pull_output();
    // ~CAS pin
    let cas = pins.gpio12.into_push_pull_output();
    // ~RAS pin
    let ras = pins.gpio13.into_push_pull_output();

    // DIN pin (into DRAM)
    let din = pins.gpio14.into_push_pull_output();

    // DOUT pin (out of DRAM) (floating as recommended by TXS0108E datasheet)
    let dout = pins.gpio15.into_floating_input();

    // TXS0108E output enable pin, start disabled
    let mut txs_oe = pins.gpio16.into_push_pull_output_in_state(PinState::Low);

    let i2c_scl = pins.gpio27.into_mode::<FunctionI2C>();
    let i2c_sda = pins.gpio26.into_mode::<FunctionI2C>();
    let i2c = I2C::new_controller(
        pac.I2C1,
        i2c_sda,
        i2c_scl,
        400u32.kHz(),
        &mut pac.RESETS,
        CLOCK.0.Hz(),
    );

    let mut display: sh1106::mode::GraphicsMode<_> = sh1106::Builder::new()
        .with_i2c_addr(0x3c)
        .with_size(sh1106::prelude::DisplaySize::Display128x64NoOffset)
        .connect_i2c(i2c)
        .into();

    display.init().unwrap();
    display.set_contrast(20).unwrap();
    display.clear();
    display.flush().unwrap();

    let char_style = MonoTextStyle::new(&mono_font::ascii::FONT_7X13, BinaryColor::On);

    let pac2 = unsafe { pac::Peripherals::steal() };

    txs_oe.set_high().unwrap();
    let mut dram = Dram41XX::new(pac2.SIO, we, cas, ras, din, dout);

    'outer: loop {
        led.set_low().unwrap();

        const CHIP_TEXT_POS: Point = Point::new(7 * 6, 0);
        const CHIP_TEXT_SIZE: Size = Size::new(7 * 7, 13);
        const CHIP_TEXT_RECT: Rectangle = Rectangle::new(CHIP_TEXT_POS, CHIP_TEXT_SIZE);

        display.clear();
        Text::with_baseline("Chip:", Point::zero(), char_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();

        loop {
            dram.init();

            if dram.is_working() {
                break;
            }

            Text::with_baseline("<none>", CHIP_TEXT_POS, char_style, Baseline::Top)
                .draw(&mut display)
                .unwrap();
            display.flush().unwrap();
        }

        let is_41256 = dram.is_41256();
        let num_addr_lines = if is_41256 { 9 } else { 8 };

        display
            .fill_solid(&CHIP_TEXT_RECT, BinaryColor::Off)
            .unwrap();
        Text::with_baseline(
            if is_41256 { "41256" } else { "4164" },
            CHIP_TEXT_POS,
            char_style,
            Baseline::Top,
        )
        .draw(&mut display)
        .unwrap();
        display.flush().unwrap();

        let mut s = heapless::String::<64>::new();
        let mut pass_count = 0u32;

        const TEST_CONTENT_POS: Point = Point::new(0, 13);
        const TEST_CONTENT_SIZE: Size = Size::new(128, 64 - 13);
        const TEST_CONTENT_RECT: Rectangle = Rectangle::new(TEST_CONTENT_POS, TEST_CONTENT_SIZE);

        loop {
            if !dram.is_working() || dram.is_41256() != is_41256 {
                // chip changed, or removed, restart
                continue 'outer;
            }

            s.clear();
            display
                .fill_solid(&TEST_CONTENT_RECT, BinaryColor::Off)
                .unwrap();
            let res = dram.test_moving_inversions(num_addr_lines);
            match res {
                Ok(()) => {
                    led.set_high().unwrap();
                    pass_count += 1;

                    info!("PASS #{}\n\n", pass_count);

                    let _ = uwrite!(&mut s, "PASS #{}", pass_count);
                    Text::with_baseline(&s, TEST_CONTENT_POS, char_style, Baseline::Top)
                        .draw(&mut display)
                        .unwrap();
                }
                Err(TestError {
                    num_failed_bits,
                    row,
                    col,
                }) => {
                    led.set_low().unwrap();
                    pass_count = 0;

                    info!(
                        "{} broken bits\nlast failed bit: row {}, col {} (bit {})\n\n",
                        num_failed_bits,
                        row,
                        col,
                        row * 256 + col
                    );

                    let _ = uwrite!(
                        &mut s,
                        "FAILS: {}\nRow {}\nCol {}\n = {:X}",
                        num_failed_bits,
                        row,
                        col,
                        row * 256 + col,
                    );
                    Text::with_baseline(&s, TEST_CONTENT_POS, char_style, Baseline::Top)
                        .draw(&mut display)
                        .unwrap();
                }
            }

            display.flush().unwrap();
        }
    }
}

struct Dram41XX<We, Cas, Ras, Din, Dout> {
    we: We,
    cas: Cas,
    ras: Ras,
    din: Din,
    dout: Dout,
    addr: AddressBus,
}

impl<We, Cas, Ras, Din, Dout> Dram41XX<We, Cas, Ras, Din, Dout>
where
    We: OutputPin,
    Cas: OutputPin,
    Ras: OutputPin,
    Din: OutputPin,
    Dout: InputPin,
{
    fn init(&mut self) {
        self.we.set_high().unwrap();
        self.cas.set_high().unwrap();
        self.ras.set_high().unwrap();

        delay_ns::<1000_0000>();

        for _ in 0..8 {
            self.ras.set_low().unwrap();
            delay_ns::<1000>();
            self.ras.set_high().unwrap();
            delay_ns::<1000>();
        }
    }

    fn is_working(&mut self) -> bool {
        self.write_one_bit_early(0, 0, false);
        if self.read_one_bit(0, 0) {
            return false;
        }
        self.write_one_bit_early(0, 0, true);
        self.read_one_bit(0, 0)
    }

    fn is_41256(&mut self) -> bool {
        self.write_one_bit_early(8, 8, false);
        self.write_one_bit_early(256 + 8, 256 + 8, true);
        if self.read_one_bit(8, 8) {
            // wrapped around → 4164
            return false;
        }

        // didn't wrap around → 41256
        true
    }

    fn test_moving_inversions(&mut self, num_addr_lines: u8) -> Result<(), TestError> {
        let addr_end = 1 << num_addr_lines;

        let mut num_failed_bits = 0;
        let mut last_failed_bit = None;
        let pat = u32::MAX;

        'test: {
            for row in 0..addr_end {
                let mut val = pat;
                self.we.set_low().unwrap();
                self.open_row(row);

                for col in 0..addr_end {
                    let bit = val & 1 != 0;

                    self.write_page_mode(col, bit);
                    val = val.rotate_right(1);
                }
                self.close_row();
                self.we.set_high().unwrap();
            }

            for row in 0..addr_end {
                self.open_row(row);
                let mut val = pat;
                for col in 0..addr_end {
                    let bit = val & 1 != 0;
                    if self.read_page_mode(col) != bit {
                        num_failed_bits += 1;
                        last_failed_bit = Some((row, col));
                    } else {
                        self.we.set_low().unwrap();
                        self.write_page_mode(col, !bit);
                        self.we.set_high().unwrap();
                    }

                    val = val.rotate_right(1);
                }
                self.close_row();
            }

            if num_failed_bits > 0 {
                break 'test;
            }

            for row in 0..addr_end {
                self.open_row(row);

                let mut val = pat;
                for col in 0..addr_end {
                    let bit = val & 1 != 0;
                    if self.read_page_mode(col) != !bit {
                        num_failed_bits += 1;
                        last_failed_bit = Some((row, col));
                    }

                    val = val.rotate_right(1);
                }
                self.close_row();
            }

            if num_failed_bits > 0 {
                break 'test;
            }

            for row in (0..addr_end).rev() {
                self.we.set_low().unwrap();
                self.open_row(row);
                let mut val = pat;
                for col in (0..addr_end).rev() {
                    let bit = val & 1 != 0;

                    self.write_page_mode(col, bit);
                    val = val.rotate_right(1);
                }
                self.close_row();
                self.we.set_high().unwrap();
            }

            for row in (0..addr_end).rev() {
                self.open_row(row);
                let mut val = pat;
                for col in (0..addr_end).rev() {
                    let bit = val & 1 != 0;
                    if self.read_page_mode(col) != bit {
                        num_failed_bits += 1;
                        last_failed_bit = Some((row, col));
                    } else {
                        self.we.set_low().unwrap();
                        self.write_page_mode(col, !bit);
                        self.we.set_high().unwrap();
                    }

                    val = val.rotate_right(1);
                }
                self.close_row()
            }

            if num_failed_bits > 0 {
                break 'test;
            }

            for row in (0..addr_end).rev() {
                self.open_row(row);
                let mut val = pat;
                for col in (0..addr_end).rev() {
                    let bit = val & 1 != 0;
                    if self.read_page_mode(col) != !bit {
                        num_failed_bits += 1;
                        last_failed_bit = Some((row, col));
                    }

                    val = val.rotate_right(1);
                }
                self.close_row();
            }

            if num_failed_bits > 0 {
                break 'test;
            }
        }

        if num_failed_bits == 0 {
            Ok(())
        } else {
            let (row, col) = last_failed_bit.unwrap();
            Err(TestError {
                num_failed_bits,
                row,
                col,
            })
        }
    }

    fn new(sio: pac::SIO, we: We, cas: Cas, ras: Ras, din: Din, dout: Dout) -> Self {
        Self {
            we,
            cas,
            ras,
            din,
            dout,
            addr: AddressBus { sio, last_state: 0 },
        }
    }

    fn write_one_bit_early(&mut self, row: usize, col: usize, bit: bool) {
        self.din.set_state(bit.into()).unwrap();
        self.we.set_low().unwrap();
        self.open_row(row);
        self.strobe_cas(col);

        self.we.set_high().unwrap();
        delay_ns::<{ Timings::T_RAS_REST }>();

        self.close_row();
    }

    fn read_one_bit(&mut self, row: usize, col: usize) -> bool {
        // read cycle
        self.open_row(row);
        self.strobe_cas(col);

        // account for bus transceiver delay
        delay_ns::<SN74HCT_DELAY>();
        let read_bit = self.dout.is_high().unwrap();
        delay_ns::<{ Timings::T_RAS_REST }>();

        self.close_row();

        read_bit
    }

    fn open_row(&mut self, row: usize) {
        self.addr.set(row);
        self.ras.set_low().unwrap();
        delay_ns::<{ Timings::T_RCD }>();
    }

    fn close_row(&mut self) {
        self.ras.set_high().unwrap();
        delay_ns::<{ Timings::T_RP }>();
    }

    fn strobe_cas(&mut self, col: usize) {
        self.addr.set(col);
        self.cas.set_low().unwrap();
        delay_ns::<{ Timings::T_CAS }>();
        self.cas.set_high().unwrap();
    }

    fn write_page_mode(&mut self, col: usize, bit: bool) {
        self.din.set_state(bit.into()).unwrap();
        self.strobe_cas(col);
        delay_ns::<{ Timings::T_CP }>();
    }

    fn read_page_mode(&mut self, col: usize) -> bool {
        self.strobe_cas(col);

        // account for bus transceiver delay
        delay_ns::<SN74HCT_DELAY>();

        let read_bit = self.dout.is_high().unwrap();
        delay_ns::<{ Timings::T_CP }>();
        read_bit
    }
}

struct TestError {
    num_failed_bits: usize,
    row: usize,
    col: usize,
}

struct AddressBus {
    sio: pac::SIO,
    last_state: u32,
}

impl AddressBus {
    #[inline(always)]
    fn set(&mut self, addr: usize) {
        let addr = addr as u32;
        self.sio.gpio_out_xor.write(|f|
                // this, in addition to the address pins being all low when AddressBus::last_state
                // is initialized to 0, ensures that we only update/change the address pins. since
                // enbedded-hal doesn't support setting multiple pins at once (and all address bits
                // should be set at the same time), we have to do this by directly writing to the
                // SIO registers instead.
                unsafe { f.bits(addr ^ self.last_state) });
        self.last_state = addr;

        delay_ns::<ADDR_SETTLE>();
    }
}
