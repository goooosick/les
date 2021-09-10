use eframe::{
    egui::{self, color::Color32, TextureId},
    epi,
};
use les::{Bus, Cpu, Ppu};

const PATTERN_SIZE: (usize, usize) = (256, 128);
const NAMETABLE_SIZE: (usize, usize) = (256, 240);
const PALETTES_SIZE: (usize, usize) = (256, 32);
const DISPLAY_SIZE: (usize, usize) = (256, 240);

struct App {
    bus: Bus,
    cpu: Cpu,

    speed: usize,
    pal_index: usize,
    nm_index: usize,

    pattern_data: Vec<u8>,
    nametable_data: Vec<u8>,
    palettes_data: Vec<u8>,
    pattern: Option<TextureId>,
    nametable: Option<TextureId>,
    palettes: Option<TextureId>,
    display: Option<TextureId>,
}

impl App {
    fn new() -> Self {
        let cart = les::Cartridge::load("nestest.nes").expect("load rom failed");
        let mut bus = les::Bus::new(cart);
        let mut cpu = les::Cpu::default();
        cpu.reset(&mut bus);

        Self {
            cpu,
            bus,

            speed: 2048,
            pal_index: 0,
            nm_index: 0,

            pattern_data: vec![0u8; PATTERN_SIZE.0 * PATTERN_SIZE.1 * 3],
            nametable_data: vec![0u8; NAMETABLE_SIZE.0 * NAMETABLE_SIZE.1 * 3],
            palettes_data: vec![0u8; PALETTES_SIZE.0 * PALETTES_SIZE.1 * 3],
            pattern: None,
            nametable: None,
            palettes: None,
            display: None,
        }
    }

    fn cpu_control(ui: &mut egui::Ui, cpu: &Cpu, cycles: usize, speed: &mut usize) -> bool {
        let s = cpu.status();
        ui.label(format!(
            "A: {:02X}    X: {:02X}    Y: {:02X}",
            s.a, s.x, s.y
        ));
        ui.label(format!("PC: {:04X}    SP: {:02X}", s.pc, s.sp));
        ui.label(format!("P: {:?}    {:02X}", s.p, s.p.to_u8()));
        ui.label(format!("CYCLES: {}", cycles));
        ui.add(egui::Slider::new(speed, 0..=10000).text("speed"));

        ui.button("RESET").clicked()
    }

    fn ppu_control(ui: &mut egui::Ui, ppu: &Ppu) {
        let t = ppu.timing();
        ui.label(format!("TIMING: ({}, {})", t.0, t.1));
    }

    fn pattern_control(ui: &mut egui::Ui, tex: &Option<TextureId>, pal_index: &mut usize) {
        if let Some(texture) = tex {
            ui.vertical_centered(|ui| {
                ui.image(*texture, (PATTERN_SIZE.0 as f32, PATTERN_SIZE.1 as f32));
                ui.add(egui::Slider::new(pal_index, 0..=7).text("palette"));
            });
        }
    }

    fn nametable_control(ui: &mut egui::Ui, tex: &Option<TextureId>, nm_index: &mut usize) {
        if let Some(texture) = tex {
            ui.vertical_centered(|ui| {
                ui.image(*texture, (NAMETABLE_SIZE.0 as f32, NAMETABLE_SIZE.1 as f32));
                ui.add(egui::Slider::new(nm_index, 0..=3).text("name table"));
            });
        }
    }

    fn palettes_control(ui: &mut egui::Ui, tex: &Option<TextureId>) {
        if let Some(texture) = tex {
            ui.centered_and_justified(|ui| {
                ui.image(*texture, (PALETTES_SIZE.0 as f32, PALETTES_SIZE.1 as f32));
            });
        }
    }

    fn render_ppu(&mut self, frame: &mut epi::Frame<'_>) {
        self.bus.ppu().render_pattern_table(
            self.bus.cart(),
            self.pattern_data.as_mut(),
            self.pal_index,
        );
        self.bus.ppu().render_name_table(
            self.bus.cart(),
            self.nametable_data.as_mut(),
            self.nm_index,
        );
        self.bus.ppu().render_palettes(self.palettes_data.as_mut());

        let data = [
            (self.pattern_data.as_ref(), PATTERN_SIZE, &mut self.pattern),
            (
                self.nametable_data.as_ref(),
                NAMETABLE_SIZE,
                &mut self.nametable,
            ),
            (
                self.palettes_data.as_ref(),
                PALETTES_SIZE,
                &mut self.palettes,
            ),
            (
                self.bus.ppu().display_buf(),
                DISPLAY_SIZE,
                &mut self.display,
            ),
        ];

        for (data, size, tex) in data {
            if let Some(tex) = tex.take() {
                frame.tex_allocator().free(tex);
            }

            *tex = Some(
                frame.tex_allocator().alloc_srgba_premultiplied(
                    size,
                    data.chunks_exact(3)
                        .map(|c| Color32::from_rgb(c[0], c[1], c[2]))
                        .collect::<Vec<_>>()
                        .as_ref(),
                ),
            );
        }
    }
}

impl epi::App for App {
    fn name(&self) -> &str {
        "LES"
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        self.render_ppu(frame);

        let Self {
            cpu,
            bus,
            speed,
            pal_index,
            nm_index,
            pattern,
            nametable,
            palettes,
            display,
            ..
        } = self;
        for _ in 0..*speed {
            cpu.exec(bus);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.heading("Display");
                if let Some(tex) = display {
                    ui.image(*tex, (DISPLAY_SIZE.0 as f32, DISPLAY_SIZE.1 as f32));
                }
            });
        });

        egui::SidePanel::left("left")
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("CPU");
                });
                if Self::cpu_control(ui, cpu, bus.cycles(), speed) {
                    cpu.reset(bus);
                }

                ui.vertical_centered(|ui| {
                    ui.heading("PPU");
                });
                Self::ppu_control(ui, bus.ppu());
            });
        egui::SidePanel::right("right")
            .resizable(false)
            .default_width(256.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Pattern Table");
                });
                Self::pattern_control(ui, pattern, pal_index);

                ui.vertical_centered(|ui| {
                    ui.heading("Nametable");
                });
                Self::nametable_control(ui, nametable, nm_index);

                ui.vertical_centered(|ui| {
                    ui.heading("Palettes");
                });
                Self::palettes_control(ui, palettes);
            });
    }
}

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some((900.0, 525.0).into()),
        ..Default::default()
    };
    eframe::run_native(Box::new(App::new()), options);
}
