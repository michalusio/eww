# egui-winit

[![Crates.io](https://img.shields.io/crates/v/egui-winit.svg)](https://crates.io/crates/egui-winit)
[![Docs.rs](https://docs.rs/egui-winit/badge.svg)](https://docs.rs/egui-winit)

egui-winit is a platform support crate for [egui](https://github.com/emilk/egui)
and [winit](https://github.com/rust-windowing/winit).

It is build as part of [eww](https://github.com/LU15W1R7H/eww), which takes egui-winit and
combines it with [egui-wgpu](https://github.com/LU15W1R7H/eww/tree/main/egui-wgpu), a render support crate for egui and wgpu.

If you want to use the winit-wgpu combination, then using eww is recommened.
If you want to use a different renderer, then you can combine it with egui-winit.

## Status

egui-winit is in early development, like eww and egui-wgpu. Therefore all crates are at `v0.0.1-alpha.x`.

## Contribution

egui-winit is an [egui\_winit\_platform](https://github.com/hasenbanck/egui_winit_platform) fork.

Feel free to contribute to this project. Just keep the Guidelines in mind.

## Guidelines

We're aiming to have and keep feature parity
with [`egui_glium`](https://github.com/emilk/egui/tree/master/egui_glium) and extend it.

egui-winit should be pretty barebones in contrast to eww since it is meant to build upon.

