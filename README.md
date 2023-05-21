# 4164 and 41256 DRAM tester for RPi Pico

The project setup is based on the
[rp2040-project-template](https://github.com/rp-rs/rp2040-project-template).

## Features

- Automatically detects 41256 vs 4164 DRAM
- Moving inversion testing (as explained over
  [here](https://www.memtest86.com/tech_memtest-algoritm.html))
- An attempt at a relatively accurate timing control with multiple speed presets, see the `Timings`
  type and `timings.rs` (has to be chosen at compile time)
- Text output on a SH1106 128x64 OLED display

The 74HCT244 can be replaced with a 74HCT245 (which I have done since I didn't have any 244s), just
make sure to pull the direction pin correctly.

## Disclaimer

I'm a total noob when it comes to electronics, so please feel free to let me know if I'm doing
something stupid :^).
