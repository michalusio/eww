# eww

eww (egui + winit + wgpu) is a egui backend with a winit platform and a wgpu renderer.

This repository contains
- the `egui-winit` crate providing egui platform support for winit,
- the `egui-wgpu` crate providing egui rendering support for wgpu, and
- the `egui` crate builds on top of the winit and wgpu support crates.

## Status

eww is in early development. Therefore all crates are at `v0.0.0-alpha.x`

egui-{winit, wgpu} are longer around and have higher version numbers.

## eww crate

Furthermore is contains
- the `eww` crate which is build on top of `egui-winit` and `egui-wgpu`
  and provides a more convenient API.

For just using the winit-wgpu combination, eww is recommended.

## egui-{winit, wgpu} crates

If you're intrested in building your own backend then you can either use the
- egui-winit combined with a different renderer, or
- egui-wgpu combined with a different platform.

## Contribution

egui-winit is an [`egui_winit_platform`](https://github.com/hasenbanck/egui_winit_platform) fork and  
egui-wgpu an [`egui_wgpu_backend`](https://github.com/hasenbanck/egui_wgpu_backend) fork.

Feel free to contribute to this project. Just keep the Guidelines in mind.

## (vague) Guidelines

We're aiming to have and keep feature parity
with [`egui_glium`](https://github.com/emilk/egui/tree/master/egui_glium) and extend it.

eww should be simple-to-use and handle the interaction between winit and wgpu.
egui-{winit, wgpu} should be more barebones since they are meant to built upon.

