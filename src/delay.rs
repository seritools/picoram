//! High-accuracy delay functions.

use core::{arch::asm, sync::atomic::compiler_fence};

/// Blocks the program for `NS` nanoseconds (+ up to 1 cycle).
#[inline(always)]
pub fn delay_ns<const NS: u32, const SYSTEM_FREQ: u32>() {
    // don't let the compiler reorder the delay loop
    compiler_fence(core::sync::atomic::Ordering::SeqCst);

    let mut ns_per_cycle: u32 = 1_000_000_000 / SYSTEM_FREQ;
    if 1_000_000_000 % SYSTEM_FREQ != 0 {
        ns_per_cycle += 1;
    }

    let cycles = NS / ns_per_cycle;
    let rest = NS % ns_per_cycle;

    let loop_count = cycles / 3;
    let rest_cycles = cycles % 3;

    if loop_count > 0 {
        delay_loop_3cyc(loop_count);
    }

    match rest_cycles {
        0 => {}
        1 => {
            nop();
        }
        2 => {
            nop();
            nop();
        }
        _ => unsafe {
            core::hint::unreachable_unchecked();
        },
    }

    if rest > 0 {
        // make sure we never under-delay
        nop();
    }

    // don't let the compiler reorder the delay loop
    compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

/// Blocks the program for 3 * `loop_count` CPU cycles.
#[inline(always)]
pub fn delay_loop_3cyc(loop_count: u32) {
    // Cortex-M0+: 1 cycle to set the register for the loop count, then
    // 3 cycles per iteration if loop continues, 2 if it breaks.
    //
    // → exactly 3 cycles per iteration
    unsafe {
        asm!(
            // Use local labels to avoid R_ARM_THM_JUMP8 relocations which fail on thumbv6m.
            "1:",
            "subs {}, #1", // 1 cycle
            "bne 1b", // 2 cycles if loop continues, 1 if not
            inout(reg) loop_count => _,
            options(nomem, nostack),
        )
    };
}

/// Blocks the program for one CPU cycles.
#[inline(always)]
fn nop() {
    unsafe { asm!("nop", options(nomem, nostack)) };
}
