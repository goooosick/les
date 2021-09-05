use eframe::{
    egui::{self, color::Color32, TextureId},
    epi,
};
use les::{Bus, Cartridge, Cpu, Ppu};

const PATTERN_SIZE: (usize, usize) = (256, 128);

struct App {
    bus: Bus,
    cpu: Cpu,

    speed: usize,
    pal_index: usize,

    pattern_data: Vec<u8>,
    pattern: Option<TextureId>,
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

            speed: 100,
            pal_index: 0,

            pattern_data: vec![0u8; PATTERN_SIZE.0 * PATTERN_SIZE.1 * 3],
            pattern: None,
        }
    }

    fn cpu_control(ui: &mut egui::Ui, cpu: &Cpu, cycles: usize, speed: &mut usize) {
        let s = cpu.status();
        ui.label(format!("A: {:02X}", s.a));
        ui.label(format!("X: {:02X}", s.x));
        ui.label(format!("Y: {:02X}", s.y));
        ui.label(format!("SP: {:02X}", s.sp));
        ui.label(format!("PC: {:04X}", s.pc));
        ui.label(format!("P: {:?}", s.p));
        ui.label(format!("CYCLES: {:}", cycles));
        ui.add(egui::Slider::new(speed, 0..=1024).text("speed"));
    }

    fn pattern_control(ui: &mut egui::Ui, tex: &Option<TextureId>, pal_index: &mut usize) {
        if let Some(texture) = tex {
            ui.vertical_centered(|ui| {
                ui.add(egui::Slider::new(pal_index, 0..=3).text("palette"));
                ui.image(*texture, (PATTERN_SIZE.0 as f32, PATTERN_SIZE.1 as f32));
            });
        }
    }

    fn render_pattern(&mut self, frame: &mut epi::Frame<'_>) {
        self.bus.ppu().render_pattern_table(
            self.bus.cart(),
            self.pattern_data.as_mut(),
            self.pal_index,
        );

        if let Some(tex) = self.pattern.take() {
            frame.tex_allocator().free(tex);
        }

        let tex = frame.tex_allocator().alloc_srgba_premultiplied(
            PATTERN_SIZE,
            self.pattern_data
                .chunks_exact(3)
                .map(|c| Color32::from_rgb(c[0], c[1], c[2]))
                .collect::<Vec<_>>()
                .as_ref(),
        );
        self.pattern = Some(tex);
    }
}

impl epi::App for App {
    fn name(&self) -> &str {
        "LES"
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        self.render_pattern(frame);

        let Self {
            cpu,
            bus,
            speed,
            pal_index,
            pattern,
            ..
        } = self;
        for _ in 0..*speed {
            cpu.exec(bus);
        }

        egui::CentralPanel::default().show(ctx, |ui| {});

        egui::SidePanel::left("left")
            .resizable(false)
            .default_width(256.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Cpu");
                });
                Self::cpu_control(ui, cpu, bus.cycles(), speed);

                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    ui.heading("Pattern Table");
                });
                Self::pattern_control(ui, pattern, pal_index);
            });
    }
}

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(App::new()), options);
}
