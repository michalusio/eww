//! [egui](https://github.com/emilk/egui) platform support for [winit](https://github.com/rust-windowing/winit)

use egui::{math::vec2, paint::ClippedShape, CtxRef, Pos2};
use winit::event::Event;

use copypasta::{ClipboardContext, ClipboardProvider as _};
use tracing::error;

use std::hash::{Hash, Hasher};

const SCROLL_LINE: f32 = 24.0;

/// Platform creation descriptor
pub struct PlatformDescriptor<'a> {
    /// Winit window
    pub window: &'a winit::window::Window,
    /// Egui style configuration.
    pub style: egui::Style,
    /// Egui font configuration.
    pub font_definitions: egui::FontDefinitions,
}

/// egui platform support for winit.
pub struct Platform {
    context: CtxRef,

    raw_input: egui::RawInput,
    pointer_pos: egui::Pos2,
    modifier_state: winit::event::ModifiersState,
    start_instant: std::time::Instant,

    scale_factor: f64,
    clipboard: Option<ClipboardContext>,
}

// public API
impl Platform {
    /// Create a new [`Platform`].
    pub fn new(desc: PlatformDescriptor) -> Self {
        let context = CtxRef::default();
        context.set_style(desc.style);
        context.set_fonts(desc.font_definitions);

        let pointer_pos = Default::default();
        let modifier_state = winit::event::ModifiersState::empty();
        let start_instant = std::time::Instant::now();
        let scale_factor = desc.window.scale_factor();
        let clipboard = Self::init_clipboard();

        let raw_input = egui::RawInput {
            pixels_per_point: Some(scale_factor as f32),
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                util::translate::vec_w2e(desc.window.inner_size().to_logical(scale_factor)),
            )),
            ..Default::default()
        };

        Self {
            context,

            raw_input,
            pointer_pos,
            modifier_state,
            start_instant,

            scale_factor,
            clipboard,
        }
    }

    /// Handles winit events and passes them to egui.
    pub fn handle_event<T>(&mut self, event: &Event<T>) -> bool {
        use winit::event::WindowEvent::*;

        if let Event::WindowEvent { event, .. } = event {
            match event {
                Resized(physical_size) => {
                    self.raw_input.screen_rect = Some(egui::Rect::from_min_size(
                        Pos2::ZERO,
                        vec2(physical_size.width as f32, physical_size.height as f32)
                            / self.scale_factor as f32,
                    ));
                    false
                }
                ScaleFactorChanged {
                    scale_factor,
                    new_inner_size,
                } => {
                    self.scale_factor = *scale_factor;
                    self.raw_input.pixels_per_point = Some(*scale_factor as f32);
                    self.raw_input.screen_rect = Some(egui::Rect::from_min_size(
                        Pos2::ZERO,
                        vec2(new_inner_size.width as f32, new_inner_size.height as f32)
                            / self.scale_factor as f32,
                    ));
                    false
                }
                MouseInput { state, button, .. } => {
                    if let Some(button) = util::translate::mouse_button_w2e(*button) {
                        self.raw_input.events.push(egui::Event::PointerButton {
                            pos: self.pointer_pos,
                            button,
                            pressed: matches!(*state, winit::event::ElementState::Pressed),
                            modifiers: Default::default(),
                        });
                    }
                    false
                }
                MouseWheel { delta, .. } => match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        self.raw_input.scroll_delta = vec2(*x, *y) * SCROLL_LINE;
                        self.context().wants_pointer_input()
                    }
                    winit::event::MouseScrollDelta::PixelDelta(delta) => {
                        self.raw_input.scroll_delta = vec2(delta.x as f32, delta.y as f32);
                        self.context().wants_pointer_input()
                    }
                },
                Touch(touch) => {
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    touch.device_id.hash(&mut hasher);
                    self.raw_input.events.push(egui::Event::Touch {
                        device_id: egui::TouchDeviceId(hasher.finish()),
                        id: egui::TouchId::from(touch.id),
                        phase: match touch.phase {
                            winit::event::TouchPhase::Started => egui::TouchPhase::Start,
                            winit::event::TouchPhase::Moved => egui::TouchPhase::Move,
                            winit::event::TouchPhase::Ended => egui::TouchPhase::End,
                            winit::event::TouchPhase::Cancelled => egui::TouchPhase::Cancel,
                        },
                        pos: util::translate::pos_w2e(touch.location.to_logical(self.scale_factor)),
                        force: match touch.force {
                            Some(winit::event::Force::Normalized(force)) => force as f32,
                            Some(winit::event::Force::Calibrated {
                                force,
                                max_possible_force,
                                ..
                            }) => (force / max_possible_force) as f32,
                            None => 0_f32,
                        },
                    });
                    true
                }
                CursorMoved { position, .. } => {
                    self.pointer_pos =
                        util::translate::pos_w2e(position.to_logical(self.scale_factor));
                    self.raw_input
                        .events
                        .push(egui::Event::PointerMoved(self.pointer_pos));
                    self.context().is_using_pointer()
                }
                CursorLeft { .. } => {
                    self.raw_input.events.push(egui::Event::PointerGone);
                    false
                }
                ModifiersChanged(input) => {
                    self.modifier_state = *input;
                    self.context().wants_keyboard_input()
                }
                KeyboardInput {
                    input:
                        winit::event::KeyboardInput {
                            virtual_keycode: Some(key),
                            state,
                            ..
                        },
                    ..
                } => {
                    if let Some(event) = self.handle_key(*key, *state) {
                        self.raw_input.events.push(event);
                    }
                    self.context().wants_keyboard_input()
                }
                ReceivedCharacter(ch) => {
                    if util::is_egui_printable(*ch)
                        && !self.modifier_state.ctrl()
                        && !self.modifier_state.logo()
                    {
                        self.raw_input
                            .events
                            .push(egui::Event::Text(ch.to_string()));
                    }
                    self.context().wants_keyboard_input()
                }
                _ => false,
            }
        } else {
            false
        }
    }

    /// Starts a new frame.
    pub fn begin_frame(&mut self) {
        self.raw_input.time = Some(self.start_instant.elapsed().as_secs_f64());

        self.context.begin_frame(self.raw_input.take());
    }

    /// Ends the frame.
    /// Returns the shapes to tessellate and draw and whetever a repaint is needed or not.
    pub fn end_frame(&mut self, window: &winit::window::Window) -> (Vec<ClippedShape>, bool) {
        let (
            egui::Output {
                cursor_icon,
                open_url,
                copied_text,
                needs_repaint,
                events: _,
                text_cursor: _,
            },
            shapes,
        ) = self.context.end_frame();
        Self::handle_cursor_icon(cursor_icon, window);
        Self::handle_copied_text(copied_text, self.clipboard.as_mut());
        Self::handle_url(open_url);

        (shapes, needs_repaint)
    }

    /// Returns the internal egui context.
    pub fn context(&self) -> CtxRef {
        self.context.clone()
    }
}

// private implementation
impl Platform {
    fn init_clipboard() -> Option<ClipboardContext> {
        match ClipboardContext::new() {
            Ok(c) => Some(c),
            Err(e) => {
                error!("Failed to initalize clipboard support: {}", e);
                None
            }
        }
    }

    fn handle_key(
        &mut self,
        key: winit::event::VirtualKeyCode,
        state: winit::event::ElementState,
    ) -> Option<egui::Event> {
        use winit::event::VirtualKeyCode;
        match key {
            VirtualKeyCode::Copy => Some(egui::Event::Copy),
            VirtualKeyCode::Cut => Some(egui::Event::Cut),
            VirtualKeyCode::Paste => self
                .clipboard
                .as_mut()
                .and_then(|c| match c.get_contents() {
                    Ok(c) => Some(c),
                    Err(e) => {
                        error!("Failed to get clipboard contents: {}", e);
                        None
                    }
                })
                .map(egui::Event::Text),
            key => util::translate::key_w2e(key).map(|key| egui::Event::Key {
                key,
                pressed: matches!(state, winit::event::ElementState::Pressed),
                modifiers: util::translate::modifiers_w2e(self.modifier_state),
            }),
        }
    }

    fn handle_cursor_icon(cursor_icon: egui::CursorIcon, window: &winit::window::Window) {
        window.set_cursor_icon(util::translate::cursor_icon_e2w(cursor_icon));
    }

    fn handle_copied_text(copied_text: String, clipboard: Option<&mut ClipboardContext>) {
        if !copied_text.is_empty() {
            if let Some(clipboard) = clipboard {
                if let Err(err) = clipboard.set_contents(copied_text) {
                    error!("Failed to set clipoard contents: {}", err);
                }
            }
        }
    }

    fn handle_url(url: Option<egui::output::OpenUrl>) {
        if let Some(url) = url {
            // TODO: use `url.new_tab`
            if let Err(err) = webbrowser::open(&url.url) {
                error!("Failed to open url: {}", err);
            }
        }
    }
}

/// Utilities for working with egui and winit together.
pub mod util {
    /// Check if egui can print a character.
    pub fn is_egui_printable(chr: char) -> bool {
        let is_in_private_use_area = ('\u{e000}'..='\u{f8ff}').contains(&chr)
            || ('\u{f0000}'..='\u{ffffd}').contains(&chr)
            || ('\u{100000}'..='\u{10fffd}').contains(&chr);

        !is_in_private_use_area && !chr.is_ascii_control()
    }

    /// Translating types between egui and winit.
    ///
    /// **e2w**: egui => winit  
    /// **w2e**: winit => egui
    pub mod translate {
        /// Translate keycode from winit to egui.
        pub fn key_w2e(w: winit::event::VirtualKeyCode) -> Option<egui::Key> {
            use egui::Key as E;
            use winit::event::VirtualKeyCode as W;
            Some(match w {
                W::Down => E::ArrowDown,
                W::Left => E::ArrowLeft,
                W::Right => E::ArrowRight,
                W::Up => E::ArrowUp,

                W::Escape => E::Escape,
                W::Tab => E::Tab,
                W::Back => E::Backspace,
                W::Return => E::Enter,
                W::Space => E::Insert,
                W::Delete => E::Delete,
                W::Home => E::Home,
                W::End => E::End,
                W::PageUp => E::PageUp,
                W::PageDown => E::PageDown,

                W::Key1 | W::Numpad1 => E::Num1,
                W::Key2 | W::Numpad2 => E::Num2,
                W::Key3 | W::Numpad3 => E::Num3,
                W::Key4 | W::Numpad4 => E::Num4,
                W::Key5 | W::Numpad5 => E::Num5,
                W::Key6 | W::Numpad6 => E::Num6,
                W::Key7 | W::Numpad7 => E::Num7,
                W::Key8 | W::Numpad8 => E::Num8,
                W::Key9 | W::Numpad9 => E::Num9,
                W::Key0 | W::Numpad0 => E::Num0,

                W::A => E::A,
                W::B => E::B,
                W::C => E::C,
                W::D => E::D,
                W::E => E::E,
                W::F => E::F,
                W::G => E::G,
                W::H => E::H,
                W::I => E::I,
                W::J => E::J,
                W::K => E::K,
                W::L => E::L,
                W::M => E::M,
                W::N => E::N,
                W::O => E::O,
                W::P => E::P,
                W::Q => E::Q,
                W::R => E::R,
                W::S => E::S,
                W::T => E::T,
                W::U => E::U,
                W::V => E::V,
                W::W => E::W,
                W::X => E::X,
                W::Y => E::Y,
                W::Z => E::Z,

                _ => {
                    return None;
                }
            })
        }

        /// Translate modifier keys from winit to egui.
        pub fn modifiers_w2e(w: winit::event::ModifiersState) -> egui::Modifiers {
            egui::Modifiers {
                alt: w.alt(),
                ctrl: w.ctrl(),
                shift: w.shift(),
                mac_cmd: if cfg!(target_os = "macos") {
                    w.logo()
                } else {
                    false
                },
                command: if cfg!(target_os = "macos") {
                    w.logo()
                } else {
                    w.ctrl()
                },
            }
        }

        /// Translate mouse button from winit to egui.
        pub fn mouse_button_w2e(w: winit::event::MouseButton) -> Option<egui::PointerButton> {
            use egui::PointerButton as E;
            use winit::event::MouseButton as W;
            Some(match w {
                W::Left => E::Primary,
                W::Right => E::Secondary,
                W::Middle => E::Middle,
                W::Other(_) => {
                    return None;
                }
            })
        }

        /// Translate cursor icon from egui to winit.
        pub fn cursor_icon_e2w(e: egui::CursorIcon) -> winit::window::CursorIcon {
            use egui::CursorIcon as E;
            use winit::window::CursorIcon as W;

            match e {
                E::Default => W::Default,
                E::None => W::Default, // TODO: handle this case properly
                E::ContextMenu => W::ContextMenu,
                E::Help => W::Help,
                E::PointingHand => W::Hand,
                E::Progress => W::Progress,
                E::Wait => W::Wait,
                E::Cell => W::Cell,
                E::Crosshair => W::Crosshair,
                E::Text => W::Text,
                E::VerticalText => W::VerticalText,
                E::Alias => W::Alias,
                E::Copy => W::Copy,
                E::Move => W::Move,
                E::NoDrop => W::NoDrop,
                E::NotAllowed => W::NotAllowed,
                E::Grab => W::Grab,
                E::Grabbing => W::Grabbing,
                E::AllScroll => W::AllScroll,
                E::ResizeHorizontal => W::ColResize,
                E::ResizeNeSw => W::NeswResize,
                E::ResizeNwSe => W::NwseResize,
                E::ResizeVertical => W::RowResize,
                E::ZoomIn => W::ZoomIn,
                E::ZoomOut => W::ZoomOut,
            }
        }

        /// Translate screen position from winit to egui.
        ///
        /// This function assumnes that the egui screen rect coincides with the winit screen screen
        /// rect.
        pub fn pos_w2e(w: winit::dpi::LogicalPosition<f32>) -> egui::Pos2 {
            egui::pos2(w.x, w.y)
        }

        /// Translate screen position from egui to winit.
        ///
        /// This function assumnes that the egui screen rect coincides with the winit screen screen
        /// rect.
        pub fn pos_e2w(e: egui::Pos2) -> winit::dpi::LogicalPosition<f32> {
            winit::dpi::LogicalPosition::new(e.x, e.y)
        }

        /// Translate screen vector/size from winit to egui.
        ///
        /// This function assumnes that the egui screen rect coincides with the winit screen screen
        /// rect.
        pub fn vec_w2e(w: winit::dpi::LogicalSize<f32>) -> egui::Vec2 {
            egui::vec2(w.width, w.height)
        }

        /// Translate screen vector/size from egui to winit.
        ///
        /// This function assumnes that the egui screen rect coincides with the winit screen screen
        /// rect.
        pub fn vec_e2w(e: egui::Vec2) -> winit::dpi::LogicalSize<f32> {
            winit::dpi::LogicalSize::new(e.x, e.y)
        }
    }
}
