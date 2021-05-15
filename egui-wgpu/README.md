# egui-wgpu

[![Crates.io](https://img.shields.io/crates/v/egui-wgpu.svg)](https://crates.io/crates/egui-wgpu)
[![Docs.rs](https://docs.rs/egui-wgpu/badge.svg)](https://docs.rs/egui-wgpu)

egui-wgpu is a render support crate for [egui](https://github.com/emilk/egui)
and [wgpu](https://github.com/gfx-rs/wgpu-rs).

It is build as part of [eww](https://github.com/LU15W1R7H/eww), which takes egui-wgpu and
combines it with [egui-winit](https://github.com/LU15W1R7H/eww/tree/main/egui-winit), a platform support crate for egui and winit.

If you want to use the winit-wgpu combination, then using eww is recommened.
If you want to use a different platform, then you can combine it with egui-wgpu.

## Status

egui-wgpu is in early development, like eww and egui-winit. Therefore all crates are at `v0.0.1-alpha.x`.

## Contribution

egui-wgpu is an [egui\_wgpu\_backend](https://github.com/hasenbanck/egui_winit_backend) fork.

Feel free to contribute to this project. Just keep the Guidelines in mind.

## Guidelines

We're aiming to have and keep feature parity
with [`egui_glium`](https://github.com/emilk/egui/tree/master/egui_glium) and extend it.

egui-wgpu should be pretty barebones in contrast to eww since it is meant to build upon.

