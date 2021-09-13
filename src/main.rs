use eframe::{
    egui::{self, color::Color32, InputState, Key, TextureId},
    epi,
};
use les::{Bus, Cpu};

const PATTERN_SIZE: (usize, usize) = (256, 128);
const NAMETABLE_SIZE: (usize, usize) = (256, 240);
const PALETTES_SIZE: (usize, usize) = (256, 32);
const DISPLAY_SIZE: (usize, usize) = (256, 240);
const SPRITES_SIZE: (usize, usize) = (256, 16);

const CYCLES_PER_FRAME: usize = 21441960 / 12 / 60;

struct ImageTexture {
    id: Option<TextureId>,
    size: (usize, usize),
    data: Vec<Color32>,
}

struct App {
    bus: Bus,
    cpu: Cpu,

    step: bool,
    pause: bool,
    speed: usize,
    pal_index: usize,
    nm_index: usize,
    scale: f32,

    textures: Vec<ImageTexture>,
}

impl App {
    fn new() -> Self {
        let cart = les::Cartridge::load("nestest.nes").expect("load rom failed");
        let mut bus = les::Bus::new(cart);
        let mut cpu = les::Cpu::default();
        cpu.reset(&mut bus);

        let mut textures = Vec::new();
        for size in [
            PATTERN_SIZE,
            NAMETABLE_SIZE,
            PALETTES_SIZE,
            SPRITES_SIZE,
            DISPLAY_SIZE,
        ] {
            textures.push(ImageTexture {
                id: None,
                size,
                data: vec![Color32::BLACK; size.0 * size.1],
            });
        }

        Self {
            cpu,
            bus,

            step: false,
            pause: false,
            speed: CYCLES_PER_FRAME,
            pal_index: 0,
            nm_index: 0,
            scale: 1.0,

            textures,
        }
    }

    fn left_panel(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("CPU");
        });
        {
            let s = self.cpu.status();
            ui.label(format!(
                "A: {:02X}    X: {:02X}    Y: {:02X}",
                s.a, s.x, s.y
            ));
            ui.label(format!("PC: {:04X}    SP: {:02X}", s.pc, s.sp));
            ui.label(format!("P: {:?}    {:02X}", s.p, s.p.to_u8()));
            ui.label(format!("CYCLES: {}", self.bus.cycles()));
            ui.add(egui::Slider::new(&mut self.speed, 0..=(CYCLES_PER_FRAME * 2)).text("speed"));
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("RESET").clicked() || ui.input().key_pressed(Key::R) {
                    self.cpu.reset(&mut self.bus);
                    self.speed = CYCLES_PER_FRAME;
                }
                if ui.button("STEP").clicked() || ui.input().key_pressed(Key::S) {
                    self.pause = true;
                    self.step = true;
                }
                if ui
                    .button(["PAUSE", "CONTINUE"][self.pause as usize])
                    .clicked()
                    || ui.input().key_pressed(Key::A)
                {
                    self.pause = !self.pause;
                }
            });
            ui.separator();
        }

        ui.vertical_centered(|ui| {
            ui.heading("PPU");
        });
        {
            let t = self.bus.ppu().timing();
            ui.label(format!("TIMING: ({}, {})", t.0, t.1));
            ui.label(format!("FRAME TIME: {}", ui.input().unstable_dt * 1000.0));
            ui.add(
                egui::Slider::new(&mut self.scale, 1.0..=3.0)
                    .clamp_to_range(true)
                    .text("scale"),
            );
        }
    }

    fn right_panel(&mut self, ui: &mut egui::Ui) {
        let headings = ["Pattern Table", "Nametable", "Palettes", "Sprites"];

        for (i, heading) in headings.iter().enumerate() {
            ui.vertical_centered(|ui| {
                ui.heading(*heading);
            });
            if let Some(texture) = self.textures[i].id {
                ui.vertical_centered(|ui| {
                    let size = self.textures[i].size;
                    ui.image(texture, (size.0 as f32, size.1 as f32));

                    if i == 0 {
                        ui.add(egui::Slider::new(&mut self.pal_index, 0..=7).text("palette"));
                    }
                    if i == 1 {
                        ui.add(egui::Slider::new(&mut self.nm_index, 0..=3).text("name table"));
                    }
                });
            }
            ui.separator();
        }
    }

    fn render_ppu(&mut self, frame: &mut epi::Frame<'_>) {
        let ppu = self.bus.ppu();
        let cart = self.bus.cart();

        ppu.render_pattern_table(cart, self.textures[0].data.as_mut(), self.pal_index);
        ppu.render_name_table(cart, self.textures[1].data.as_mut(), self.nm_index);
        ppu.render_palettes(self.textures[2].data.as_mut());
        ppu.render_sprites(cart, self.textures[3].data.as_mut());
        ppu.render_display(self.textures[4].data.as_mut());

        for im in self.textures.iter_mut() {
            if let Some(tex) = im.id.take() {
                frame.tex_allocator().free(tex);
            }

            im.id = Some(
                frame
                    .tex_allocator()
                    .alloc_srgba_premultiplied(im.size, &im.data),
            );
        }
    }

    fn collect_input(input: &InputState) -> (les::InputStates, les::InputStates) {
        (
            les::InputStates {
                a: input.key_down(Key::Z),
                b: input.key_down(Key::X),
                select: input.key_down(Key::C),
                start: input.key_down(Key::V),
                up: input.key_down(Key::ArrowUp),
                down: input.key_down(Key::ArrowDown),
                left: input.key_down(Key::ArrowLeft),
                right: input.key_down(Key::ArrowRight),
            },
            Default::default(),
        )
    }

    fn update_emu(&mut self, dt: f32, inputs: (les::InputStates, les::InputStates)) {
        self.bus.set_input0(inputs.0);
        self.bus.set_input1(inputs.1);

        if !self.pause {
            let cycles = (dt / (1.0 / 60.0) * (self.speed as f32)) as usize;
            let end = self.bus.cycles() + cycles;
            while self.bus.cycles() < end {
                self.bus.exec(&mut self.cpu);
            }
        } else {
            if self.step {
                self.bus.exec(&mut self.cpu);
                self.step = false;
            }
        }
    }
}

impl epi::App for App {
    fn name(&self) -> &str {
        "LES"
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        let input = ctx.input();

        self.update_emu(input.unstable_dt, Self::collect_input(input));
        self.render_ppu(frame);

        egui::SidePanel::left("left")
            .resizable(false)
            .default_width(256.0)
            .show(ctx, |ui| {
                self.left_panel(ui);
            });
        egui::SidePanel::right("right")
            .resizable(false)
            .default_width(256.0)
            .show(ctx, |ui| {
                self.right_panel(ui);
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.heading("Display");
                let im = &self.textures[4];
                if let Some(tex) = im.id {
                    ui.image(
                        tex,
                        (im.size.0 as f32 * self.scale, im.size.1 as f32 * self.scale),
                    );
                }
            });
        });
    }
}

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some((900.0, 600.0).into()),
        ..Default::default()
    };
    eframe::run_native(Box::new(App::new()), options);
}
