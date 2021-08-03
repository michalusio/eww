# eww

[![Crates.io](https://img.shields.io/crates/v/eww.svg)](https://crates.io/crates/eww)
[![Docs.rs](https://docs.rs/eww/badge.svg)](https://docs.rs/eww)

eww (egui + winit + wgpu) is a [egui](https://github.com/emilk/egui) backend with
a [winit](https://github.com/rust-windowing/winit) platform and a [wgpu](https://github.com/gfx-rs/wgpu-rs) renderer.

This repository contains:
- the egui-winit crate providing egui platform support for winit,
- the egui-wgpu crate providing egui rendering support for wgpu, and
- the eww crate, which builds on top of the egui-{winit, wgpu}.

## Status

eww is in early development. Therefore all crates are at `v0.0.1-alpha.x`

## eww crate

The eww crate is build on top of egui-winit and egui-wgpu
  and provides a more convenient API, since it handles their interactions.

For just using the winit-wgpu combination, eww is recommended.

You can find a basic usage example under `eww/examples/basic`.

## egui-{winit, wgpu} crates

If you're intrested in building your own backend then you can either use the
- egui-winit combined with a different renderer, or
- egui-wgpu combined with a different platform.

## Contribution

egui-winit is an [egui\_winit\_platform](https://github.com/hasenbanck/egui_winit_platform) fork and  
egui-wgpu an [egui\_wgpu\_backend](https://github.com/hasenbanck/egui_wgpu_backend) fork.

Feel free to contribute to this project. Just keep the guidelines in mind.

## Guidelines

We're aiming to have and keep feature parity
with [egui\_glium](https://github.com/emilk/egui/tree/master/egui_glium) and extend it.

eww should be simple-to-use and handle the interaction between winit and wgpu.
egui-{winit, wgpu} should be more barebones since they are meant to built upon.

