use super::pac;
use fugit::{HertzU32, RateExtU32};
use rp_pico::hal::{
    clocks::{ClocksManager, InitError},
    pll::{common_configs::PLL_USB_48MHZ, setup_pll_blocking, PLLConfig},
    xosc::setup_xosc_blocking,
    Watchdog,
};

use rp_pico::hal::pll::common_configs::PLL_SYS_125MHZ;

#[allow(dead_code)]
pub const CLOCK_125: (u32, PLLConfig) = (125_000_000, PLL_SYS_125MHZ);
#[allow(dead_code)]
pub const CLOCK_150: (u32, PLLConfig) = (
    150_000_000,
    PLLConfig {
        vco_freq: HertzU32::MHz(1500),
        refdiv: 1,
        post_div1: 5,
        post_div2: 2,
    },
);
#[allow(dead_code)]
pub const CLOCK_225: (u32, PLLConfig) = (
    225_000_000,
    PLLConfig {
        vco_freq: HertzU32::MHz(900),
        refdiv: 1,
        post_div1: 4,
        post_div2: 1,
    },
);
#[allow(dead_code)]
pub const CLOCK_250: (u32, PLLConfig) = (
    250_000_000,
    PLLConfig {
        vco_freq: HertzU32::MHz(1500),
        refdiv: 1,
        post_div1: 6,
        post_div2: 1,
    },
);
#[allow(dead_code)]
pub const CLOCK_300: (u32, PLLConfig) = (
    300_000_000,
    PLLConfig {
        vco_freq: HertzU32::MHz(1500),
        refdiv: 1,
        post_div1: 5,
        post_div2: 1,
    },
);

/// Initialize the clocks and plls according to the reference implementation
#[allow(clippy::too_many_arguments)]
pub fn init_clocks_and_plls(
    pll_config: PLLConfig,
    xosc_crystal_freq: u32,
    xosc_dev: pac::XOSC,
    clocks_dev: pac::CLOCKS,
    pll_sys_dev: pac::PLL_SYS,
    pll_usb_dev: pac::PLL_USB,
    resets: &mut pac::RESETS,
    watchdog: &mut Watchdog,
) -> Result<ClocksManager, InitError> {
    let xosc = setup_xosc_blocking(xosc_dev, xosc_crystal_freq.Hz()).map_err(InitError::XoscErr)?;

    // Configure watchdog tick generation to tick over every microsecond
    watchdog.enable_tick_generation((xosc_crystal_freq / 1_000_000) as u8);

    let mut clocks = ClocksManager::new(clocks_dev);

    let pll_sys = setup_pll_blocking(
        pll_sys_dev,
        xosc.operating_frequency(),
        pll_config,
        &mut clocks,
        resets,
    )
    .map_err(InitError::PllError)?;

    let pll_usb = setup_pll_blocking(
        pll_usb_dev,
        xosc.operating_frequency(),
        PLL_USB_48MHZ,
        &mut clocks,
        resets,
    )
    .map_err(InitError::PllError)?;

    clocks
        .init_default(&xosc, &pll_sys, &pll_usb)
        .map_err(InitError::ClockError)?;

    Ok(clocks)
}
