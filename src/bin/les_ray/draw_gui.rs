use std::sync::MutexGuard;

use super::*;

impl GuiContext {
    pub(super) fn draw_gui(&mut self, mut emu: MutexGuard<EmuContext>) {
        if self.debug {
            self.render_debug_textures(&emu.bus);
        }

        let mut d = self.rl.begin_drawing(&self.thread);
        d.clear_background(Color::RAYWHITE);

        if self.debug {
            let EmuContext { cpu, bus, step, .. } = &mut *emu;
            let off_x = (DISPLAY_SIZE.0 * self.display_scale) as f32 + DEBUG_PAD_W;

            // CPU
            {
                let s = cpu.status();
                d.gui_group_box(
                    Rectangle {
                        x: off_x,
                        y: 10.0,
                        width: DEBUG_W0,
                        height: 130.0,
                    },
                    cstr!("CPU"),
                );

                // labels
                let label_bound = Rectangle {
                    x: off_x + 5.0,
                    y: 5.0,
                    width: 100.0,
                    height: 50.0,
                };
                d.gui_label(
                    label_bound,
                    cstr!("A: {:02X}    X: {:02X}    Y: {:02X}", s.a, s.x, s.y),
                );
                d.gui_label(
                    Rectangle {
                        y: label_bound.y + 20.0,
                        ..label_bound
                    },
                    cstr!("PC: {:04X}    SP: {:02X}", s.pc, s.sp),
                );
                d.gui_label(
                    Rectangle {
                        y: label_bound.y + 40.0,
                        ..label_bound
                    },
                    cstr!("P: {:?}    {:02X}", s.p, s.p.to_u8()),
                );
                d.gui_label(
                    Rectangle {
                        y: label_bound.y + 60.0,
                        ..label_bound
                    },
                    cstr!("CYCLES: {}", bus.cycles()),
                );

                // buttons
                let button_bound = Rectangle {
                    x: off_x + 5.0,
                    y: 105.0,
                    width: 80.0,
                    height: 30.0,
                };
                if d.gui_button(button_bound, cstr!("RESET")) {
                    bus.reset(cpu);
                }
                if d.gui_button(
                    Rectangle {
                        x: label_bound.x + 85.0,
                        ..button_bound
                    },
                    cstr!("STEP"),
                ) || d.is_key_pressed(KeyboardKey::KEY_S)
                {
                    self.paused = true;
                    *step = true;
                }
                if d.gui_button(
                    Rectangle {
                        x: button_bound.x + 170.0,
                        width: 120.0,
                        ..button_bound
                    },
                    cstr!("{}", if self.paused { "CONTINUE" } else { "PAUSE" }),
                ) {
                    self.paused = !self.paused;
                }
            }

            // PPU states
            {
                let t = bus.ppu().timing();
                d.gui_group_box(
                    Rectangle {
                        x: off_x,
                        y: 155.0,
                        width: DEBUG_W0,
                        height: 35.0,
                    },
                    cstr!("PPU"),
                );
                d.gui_label(
                    Rectangle {
                        x: off_x + 5.0,
                        y: 155.0,
                        width: 100.0,
                        height: 40.0,
                    },
                    cstr!("TIMING: ({}, {})", t.0, t.1),
                );
            }

            // APU
            {
                d.gui_group_box(
                    Rectangle {
                        x: off_x,
                        y: 205.0,
                        width: 300.0,
                        height: 85.0,
                    },
                    cstr!("APU"),
                );

                const NAMES: [&str; 5] = ["Pulse1", "Pulse2", "Triangle", "Noise", "DMC"];
                const OFFSET: [Vector2; 5] = [
                    Vector2 { x: 0.0, y: 0.0 },
                    Vector2 { x: 95.0, y: 0.0 },
                    Vector2 { x: 190.0, y: 0.0 },
                    Vector2 { x: 45.0, y: 35.0 },
                    Vector2 { x: 140.0, y: 35.0 },
                ];
                for (i, name) in NAMES.iter().enumerate() {
                    self.audio_ctrl[i] = d.gui_toggle(
                        Rectangle {
                            x: off_x + 11.0 + OFFSET[i].x,
                            y: 215.0 + OFFSET[i].y,
                            width: 90.0,
                            height: 30.0,
                        },
                        cstr!("{}", name),
                        self.audio_ctrl[i],
                    );
                }
                bus.set_audio_control(&self.audio_ctrl);
            }

            // pattern table
            {
                d.gui_group_box(
                    Rectangle {
                        x: off_x,
                        y: 305.0,
                        width: 300.0,
                        height: 175.0,
                    },
                    cstr!("Pattern Table"),
                );
                self.pal_index = d.gui_slider(
                    Rectangle {
                        x: off_x + 25.0,
                        y: 450.0,
                        width: 250.0,
                        height: 25.0,
                    },
                    cstr!("{}", self.pal_index),
                    None,
                    self.pal_index as f32,
                    0.0,
                    7.0,
                ) as usize;

                d.draw_texture_ex(
                    &self.debug_textures[0].0,
                    Vector2 {
                        x: off_x + 22.0,
                        y: 315.0,
                    },
                    0.0,
                    1.0,
                    Color::WHITE,
                );
            }

            // palettes
            {
                d.gui_group_box(
                    Rectangle {
                        x: off_x,
                        y: 495.0,
                        width: 300.0,
                        height: 45.0,
                    },
                    cstr!("Palettes"),
                );

                d.draw_texture_ex(
                    &self.debug_textures[5].0,
                    Vector2 {
                        x: off_x + 22.0,
                        y: 505.0,
                    },
                    0.0,
                    1.0,
                    Color::WHITE,
                );
            }

            let off_x = off_x + DEBUG_W0 + DEBUG_PAD_W;

            // sprites
            {
                d.gui_group_box(
                    Rectangle {
                        x: off_x,
                        y: 10.0,
                        width: DEBUG_W1,
                        height: 50.0,
                    },
                    cstr!("Sprites"),
                );

                d.draw_texture_ex(
                    &self.debug_textures[6].0,
                    Vector2 {
                        x: off_x + 12.0,
                        y: 23.0,
                    },
                    0.0,
                    2.0,
                    Color::WHITE,
                );
            }

            // name tables
            {
                d.gui_group_box(
                    Rectangle {
                        x: off_x,
                        y: 80.0,
                        width: DEBUG_W1,
                        height: 500.0,
                    },
                    cstr!("Nametable"),
                );

                const OFFSET: [Vector2; 4] = [
                    Vector2 { x: 0.0, y: 0.0 },
                    Vector2 { x: 258.0, y: 0.0 },
                    Vector2 { x: 0.0, y: 242.0 },
                    Vector2 { x: 258.0, y: 242.0 },
                ];
                for i in 0..4 {
                    d.draw_texture_ex(
                        &self.debug_textures[1 + i].0,
                        Vector2 {
                            x: off_x + 10.0 + OFFSET[i].x,
                            y: 92.0 + OFFSET[i].y,
                        },
                        0.0,
                        1.0,
                        Color::WHITE,
                    );
                }
            }
        }

        // must drop guard before vsync
        std::mem::drop(emu);

        d.draw_texture_ex(
            &self.render_texture,
            Vector2::default(),
            0.0,
            self.display_scale as f32,
            if self.paused {
                Color::GRAY
            } else {
                Color::WHITE
            },
        );

        if self.paused {
            d.gui_label(
                Rectangle {
                    x: 5.0,
                    y: 5.0,
                    width: 40.0,
                    height: 20.0,
                },
                cstr!("PAUSED"),
            )
        } else if self.draw_fps {
            d.draw_fps(5, 5);
        }
    }

    fn render_debug_textures(&mut self, bus: &Bus) {
        let ppu = bus.ppu();
        let cart = bus.cart();

        ppu.render_pattern_table(cart, self.debug_textures[0].1.as_mut(), self.pal_index);
        ppu.render_name_table(cart, self.debug_textures[1].1.as_mut(), 0);
        ppu.render_name_table(cart, self.debug_textures[2].1.as_mut(), 1);
        ppu.render_name_table(cart, self.debug_textures[3].1.as_mut(), 2);
        ppu.render_name_table(cart, self.debug_textures[4].1.as_mut(), 3);
        ppu.render_palettes(self.debug_textures[5].1.as_mut());
        ppu.render_sprites(cart, self.debug_textures[6].1.as_mut());

        for (tex, buf) in self.debug_textures.iter_mut() {
            tex.update_texture(
                buf.iter()
                    .flat_map(|c| [c[0], c[1], c[2], 255])
                    .collect::<Vec<u8>>()
                    .as_ref(),
            );
        }
    }
}
