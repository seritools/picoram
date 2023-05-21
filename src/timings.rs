use crate::NS_PER_CYCLE;

pub trait DramTimingConfig {
    /// Pulse duration, RAS low (ns)
    const T_RAS: u32;
    /// Pulse duration, CAS low (ns)
    const T_CAS: u32;
    /// RAS low to CAS low delay (ns)
    const T_RCD: u32;
    /// Pulse duration, RAS high (precharge) (ns)
    const T_RP: u32;
    /// Pulse duration, CAS high (precharge) (ns)
    const T_CP: u32;

    const T_RAS_REST: u32 = Self::T_RAS.saturating_sub(Self::T_CAS + Self::T_RCD);
}

pub struct Dram150Ns;
impl DramTimingConfig for Dram150Ns {
    // Timing configuration from TMS4256 datasheet
    const T_RAS: u32 = 150u32.saturating_sub(NS_PER_CYCLE);
    const T_CAS: u32 = 75u32.saturating_sub(NS_PER_CYCLE);
    const T_RCD: u32 = 25u32.saturating_sub(NS_PER_CYCLE);
    const T_RP: u32 = 100u32.saturating_sub(NS_PER_CYCLE);
    const T_CP: u32 = 60u32.saturating_sub(NS_PER_CYCLE);
}

pub struct Dram120Ns;
impl DramTimingConfig for Dram120Ns {
    // Timing configuration from TMS4256 datasheet
    const T_RAS: u32 = 120u32.saturating_sub(NS_PER_CYCLE);
    const T_CAS: u32 = 60u32.saturating_sub(NS_PER_CYCLE);
    const T_RCD: u32 = 25u32.saturating_sub(NS_PER_CYCLE);
    const T_RP: u32 = 90u32.saturating_sub(NS_PER_CYCLE);
    const T_CP: u32 = 50u32.saturating_sub(NS_PER_CYCLE);
}

pub struct Dram100Ns;
impl DramTimingConfig for Dram100Ns {
    // Timing configuration from TMS4256 datasheet
    const T_RAS: u32 = 100u32.saturating_sub(NS_PER_CYCLE);
    const T_CAS: u32 = 50u32.saturating_sub(NS_PER_CYCLE);
    const T_RCD: u32 = 25u32.saturating_sub(NS_PER_CYCLE);
    const T_RP: u32 = 90u32.saturating_sub(NS_PER_CYCLE);
    const T_CP: u32 = 40u32.saturating_sub(NS_PER_CYCLE);
}

pub struct Dram80Ns;
impl DramTimingConfig for Dram80Ns {
    // Timing configuration from TMS4256 datasheet
    const T_RAS: u32 = 80u32.saturating_sub(NS_PER_CYCLE);
    const T_CAS: u32 = 40u32.saturating_sub(NS_PER_CYCLE);
    const T_RCD: u32 = 25u32.saturating_sub(NS_PER_CYCLE);
    const T_RP: u32 = 70u32.saturating_sub(NS_PER_CYCLE);
    const T_CP: u32 = 20u32.saturating_sub(NS_PER_CYCLE);
}
