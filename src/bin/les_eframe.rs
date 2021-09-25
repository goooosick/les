use cpal::traits::*;
use eframe::{
    egui::{self, color::Color32, InputState, Key, TextureId},
    epi,
};
use les::{Bus, Cpu};
use std::sync::{Arc, Mutex};

const PATTERN_SIZE: (usize, usize) = (256, 128);
const NAMETABLE_SIZE: (usize, usize) = (256, 240);
const PALETTES_SIZE: (usize, usize) = (256, 32);
const DISPLAY_SIZE: (usize, usize) = (256, 240);
const SPRITES_SIZE: (usize, usize) = (256, 16);

fn main() {
    let cart = les::Cartridge::load("nestest.nes").expect("load rom failed");
    let emu = {
        let mut bus = les::Bus::new(cart);
        let mut cpu = les::Cpu::default();
        bus.reset(&mut cpu);

        Arc::new(Mutex::new(EmuContext {
            bus,
            cpu,
            step: false,
            pause: false,
        }))
    };

    let _stream = {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("no default device");
        let config = device.default_output_config().unwrap();

        let emu = emu.clone();
        match config.sample_format() {
            cpal::SampleFormat::I16 => run::<i16>(emu, &device, &config.into()),
            cpal::SampleFormat::U16 => run::<u16>(emu, &device, &config.into()),
            cpal::SampleFormat::F32 => run::<f32>(emu, &device, &config.into()),
        }
    };

    let options = eframe::NativeOptions {
        initial_window_size: Some((900.0, 600.0).into()),
        ..Default::default()
    };
    eframe::run_native(Box::new(App::new(emu)), options);
}

fn run<T>(
    emu: Arc<Mutex<EmuContext>>,
    device: &cpal::Device,
    config: &cpal::StreamConfig,
) -> cpal::Stream
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    let sample_step = les::CPU_FREQUENCY / sample_rate;
    let mut sample_delta = sample_step;

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let mut emu = emu.lock().unwrap();
                let EmuContext {
                    bus,
                    cpu,
                    step,
                    pause,
                } = &mut *emu;

                if *pause {
                    if *step {
                        bus.exec(cpu);
                        *step = false;
                    }

                    bus.audio_samples().clear();
                    data.fill(cpal::Sample::from(&0.0f32));
                } else {
                    let sample_len = data.len() / channels;
                    let sample_count = sample_len * sample_step.ceil() as usize;

                    while bus.apu().sample_len() < sample_count {
                        bus.exec(cpu);
                    }

                    let mut i = 0;
                    let samples = bus.audio_samples();
                    for d in data.chunks_exact_mut(channels) {
                        d.fill(cpal::Sample::from(&samples[i]));

                        i += sample_delta.trunc() as usize;
                        sample_delta = sample_delta.fract() + sample_step;
                    }
                    let _ = bus.audio_samples().drain(0..i).collect::<Vec<_>>();
                }
            },
            |err| eprintln!("an error occurred on stream: {}", err),
        )
        .unwrap();
    stream.play().unwrap();

    stream
}

struct ImageTexture {
    id: Option<TextureId>,
    size: (usize, usize),
    data: Vec<Color32>,
}

struct EmuContext {
    bus: Bus,
    cpu: Cpu,

    step: bool,
    pause: bool,
}

struct GuiContext {
    pal_index: usize,
    nm_index: usize,
    scale: f32,

    channels: [bool; 5],
    textures: Vec<ImageTexture>,
}

struct App {
    emu: Arc<Mutex<EmuContext>>,
    gui: GuiContext,
}

impl App {
    fn new(emu: Arc<Mutex<EmuContext>>) -> Self {
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
            emu,
            gui: GuiContext {
                pal_index: 0,
                nm_index: 0,
                scale: 1.0,

                channels: [true; 5],
                textures,
            },
        }
    }

    fn left_panel(ui: &mut egui::Ui, (emu, gui): (&mut EmuContext, &mut GuiContext)) {
        ui.vertical_centered(|ui| {
            ui.heading("CPU");
        });
        {
            let s = emu.cpu.status();
            ui.label(format!(
                "A: {:02X}    X: {:02X}    Y: {:02X}",
                s.a, s.x, s.y
            ));
            ui.label(format!("PC: {:04X}    SP: {:02X}", s.pc, s.sp));
            ui.label(format!("P: {:?}    {:02X}", s.p, s.p.to_u8()));
            ui.label(format!("CYCLES: {}", emu.bus.cycles()));
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("RESET").clicked() || ui.input().key_pressed(Key::R) {
                    emu.bus.reset(&mut emu.cpu);
                }
                if ui.button("STEP").clicked() || ui.input().key_pressed(Key::S) {
                    emu.pause = true;
                    emu.step = true;
                }
                if ui
                    .button(["PAUSE", "CONTINUE"][emu.pause as usize])
                    .clicked()
                    || ui.input().key_pressed(Key::A)
                {
                    emu.pause = !emu.pause;
                }
            });
        }
        ui.separator();

        ui.vertical_centered(|ui| {
            ui.heading("PPU");
        });
        {
            let t = emu.bus.ppu().timing();
            ui.label(format!("TIMING: ({}, {})", t.0, t.1));
            ui.label(format!("FRAME TIME: {}", ui.input().unstable_dt * 1000.0));
            ui.add(
                egui::Slider::new(&mut gui.scale, 1.0..=3.0)
                    .clamp_to_range(true)
                    .text("scale"),
            );
        }
        ui.separator();

        ui.vertical_centered(|ui| {
            ui.heading("APU");
        });
        {
            let mut changed = false;
            for (state, name) in gui
                .channels
                .iter_mut()
                .zip(["Pulse1", "Pulse2", "Triangle", "Noise", "DMC"])
            {
                changed |= ui.checkbox(state, name).changed();
            }
            if changed {
                emu.bus.set_audio_control(&gui.channels);
            }
        }
    }

    fn right_panel(ui: &mut egui::Ui, gui: &mut GuiContext) {
        let headings = ["Pattern Table", "Nametable", "Palettes", "Sprites"];

        for (i, heading) in headings.iter().enumerate() {
            ui.vertical_centered(|ui| {
                ui.heading(*heading);
            });
            if let Some(texture) = gui.textures[i].id {
                ui.vertical_centered(|ui| {
                    let size = gui.textures[i].size;
                    ui.image(texture, (size.0 as f32, size.1 as f32));

                    if i == 0 {
                        ui.add(egui::Slider::new(&mut gui.pal_index, 0..=7).text("palette"));
                    }
                    if i == 1 {
                        ui.add(egui::Slider::new(&mut gui.nm_index, 0..=3).text("name table"));
                    }
                });
            }
            ui.separator();
        }
    }

    fn render_ppu(frame: &mut epi::Frame<'_>, (emu, gui): (&EmuContext, &mut GuiContext)) {
        let ppu = emu.bus.ppu();
        let cart = emu.bus.cart();

        ppu.render_pattern_table(cart, gui.textures[0].data.as_mut(), gui.pal_index);
        ppu.render_name_table(cart, gui.textures[1].data.as_mut(), gui.nm_index);
        ppu.render_palettes(gui.textures[2].data.as_mut());
        ppu.render_sprites(cart, gui.textures[3].data.as_mut());
        ppu.render_display(gui.textures[4].data.as_mut());

        for im in gui.textures.iter_mut() {
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
}

impl epi::App for App {
    fn name(&self) -> &str {
        "LES"
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        let Self { emu, gui, .. } = self;
        let mut emu = emu.lock().unwrap();

        let inputs = Self::collect_input(ctx.input());
        emu.bus.set_input0(inputs.0);
        emu.bus.set_input1(inputs.1);

        Self::render_ppu(frame, (&mut emu, gui));

        egui::SidePanel::left("left")
            .resizable(false)
            .default_width(256.0)
            .show(ctx, |ui| {
                Self::left_panel(ui, (&mut emu, gui));
            });
        egui::SidePanel::right("right")
            .resizable(false)
            .default_width(256.0)
            .show(ctx, |ui| {
                Self::right_panel(ui, gui);
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.heading("Display");
                let im = &gui.textures[4];
                if let Some(tex) = im.id {
                    ui.image(
                        tex,
                        (im.size.0 as f32 * gui.scale, im.size.1 as f32 * gui.scale),
                    );
                }
            });
        });
    }
}
