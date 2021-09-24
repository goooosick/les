use cpal::traits::*;
use eframe::{egui, epi};
use les::Apu;
use std::sync::{Arc, Mutex};

mod toggle_bits;

use toggle_bits::toggle_bits;

fn main() {
    let apu = Arc::new(Mutex::new(Apu::new()));
    let _stream = {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("no default device");
        let config = device.default_output_config().unwrap();

        let apu = apu.clone();
        match config.sample_format() {
            cpal::SampleFormat::I16 => run::<i16>(apu, &device, &config.into()),
            cpal::SampleFormat::U16 => run::<u16>(apu, &device, &config.into()),
            cpal::SampleFormat::F32 => run::<f32>(apu, &device, &config.into()),
        }
    };

    let config = eframe::NativeOptions {
        initial_window_size: Some((800.0, 600.0).into()),
        ..Default::default()
    };
    eframe::run_native(Box::new(NesApu::new(apu)), config);
}

pub fn run<T>(
    apu: Arc<Mutex<Apu>>,
    device: &cpal::Device,
    config: &cpal::StreamConfig,
) -> cpal::Stream
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    let sample_step = (les::CPU_FREQUENCY / sample_rate) as usize;
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let mut apu = apu.lock().unwrap();
                data.chunks_exact_mut(channels).for_each(|s| {
                    for _ in 0..sample_step {
                        apu.tick();
                    }
                    s.fill(cpal::Sample::from(apu.samples().back().unwrap()));
                    apu.samples().clear();
                });
            },
            |err| eprintln!("an error occurred on stream: {}", err),
        )
        .unwrap();
    stream.play().unwrap();

    stream
}

struct NesApu {
    apu: Arc<Mutex<Apu>>,
    regs: [u8; 0x20],
}

impl NesApu {
    fn new(apu: Arc<Mutex<Apu>>) -> Self {
        let mut app = Self {
            apu,
            regs: Default::default(),
        };

        {
            app.regs[0x00] = 0b10000111;
            app.regs[0x01] = 0b10001001;
            app.regs[0x02] = 0b11110000;
            app.regs[0x03] = 0b00000000;

            app.regs[0x04] = 0b00111111;
            app.regs[0x05] = 0b10011010;
            app.regs[0x06] = 0b11111111;
            app.regs[0x07] = 0b00000000;

            app.regs[0x15] = 0x1f;

            let mut apu = app.apu.lock().unwrap();
            for i in 0x00..0x14 {
                apu.write(0x4000 + i, app.regs[i as usize]);
            }
            apu.write(0x4015, app.regs[0x15]);
            apu.write(0x4017, app.regs[0x17]);
        }

        app
    }

    fn write(&mut self, addr: u16, data: u8) {
        let mut apu = self.apu.lock().unwrap();
        apu.write(addr, data);
    }
}

impl epi::App for NesApu {
    fn update(&mut self, ctx: &egui::CtxRef, _: &mut epi::Frame<'_>) {
        let rect1 = egui::Window::new("Pulse 1")
            .collapsible(false)
            .anchor(egui::Align2::LEFT_TOP, (5.0, 5.0))
            .show(ctx, |ui| {
                let changes = [
                    toggle_bits(ui, &mut self.regs[0x00], ("$4000", b"DDLCVVVV", 0xff)),
                    toggle_bits(ui, &mut self.regs[0x01], ("$4001", b"EPPPNSSS", 0xff)),
                    toggle_bits(ui, &mut self.regs[0x02], ("$4002", b"TTTTTTTT", 0xff)),
                    toggle_bits(ui, &mut self.regs[0x03], ("$4003", b"LLLLLTTT", 0xff)),
                ];

                for (i, c) in changes.iter().enumerate() {
                    if *c {
                        self.write(0x4000 + i as u16, self.regs[0x00 + i]);
                    }
                }
            })
            .unwrap();

        let rect2 = egui::Window::new("Pulse 2")
            .collapsible(false)
            .anchor(
                egui::Align2::LEFT_TOP,
                rect1.response.rect.right_top().to_vec2() + (5.0, 0.0).into(),
            )
            .show(ctx, |ui| {
                let changes = [
                    toggle_bits(ui, &mut self.regs[0x04], ("$4004", b"DDLCVVVV", 0xff)),
                    toggle_bits(ui, &mut self.regs[0x05], ("$4005", b"EPPPNSSS", 0xff)),
                    toggle_bits(ui, &mut self.regs[0x06], ("$4006", b"TTTTTTTT", 0xff)),
                    toggle_bits(ui, &mut self.regs[0x07], ("$4007", b"LLLLLTTT", 0xff)),
                ];

                for (i, c) in changes.iter().enumerate() {
                    if *c {
                        self.write(0x4004 + i as u16, self.regs[0x04 + i]);
                    }
                }
            })
            .unwrap();

        let rect3 = egui::Window::new("Triangle")
            .collapsible(false)
            .anchor(
                egui::Align2::LEFT_TOP,
                rect1.response.rect.left_bottom().to_vec2() + (0.0, 5.0).into(),
            )
            .show(ctx, |ui| {
                let changes = [
                    toggle_bits(ui, &mut self.regs[0x08], ("$4008", b"CRRRRRRR", 0xff)),
                    toggle_bits(ui, &mut self.regs[0x09], ("$4009", b"--------", 0x00)),
                    toggle_bits(ui, &mut self.regs[0x0A], ("$400A", b"TTTTTTTT", 0xff)),
                    toggle_bits(ui, &mut self.regs[0x0B], ("$400B", b"LLLLLTTT", 0xff)),
                ];

                for (i, c) in changes.iter().enumerate() {
                    if *c {
                        self.write(0x4008 + i as u16, self.regs[0x08 + i]);
                    }
                }
            })
            .unwrap();

        let rect4 = egui::Window::new("Noise")
            .collapsible(false)
            .anchor(
                egui::Align2::LEFT_TOP,
                rect2.response.rect.left_bottom().to_vec2() + (0.0, 5.0).into(),
            )
            .show(ctx, |ui| {
                let changes = [
                    toggle_bits(ui, &mut self.regs[0x0C], ("$400C", b"--LCVVVV", 0x3f)),
                    toggle_bits(ui, &mut self.regs[0x0D], ("$400D", b"--------", 0x00)),
                    toggle_bits(ui, &mut self.regs[0x0E], ("$400E", b"M---PPPP", 0x8f)),
                    toggle_bits(ui, &mut self.regs[0x0F], ("$400F", b"LLLLL---", 0xf8)),
                ];

                for (i, c) in changes.iter().enumerate() {
                    if *c {
                        self.write(0x400c + i as u16, self.regs[0x0c + i]);
                    }
                }
            })
            .unwrap();

        egui::Window::new("DMC")
            .collapsible(false)
            .anchor(
                egui::Align2::LEFT_TOP,
                rect3.response.rect.left_bottom().to_vec2() + (0.0, 5.0).into(),
            )
            .show(ctx, |ui| {
                let changes = [
                    toggle_bits(ui, &mut self.regs[0x10], ("$4010", b"IL--RRRR", 0xcf)),
                    toggle_bits(ui, &mut self.regs[0x11], ("$4011", b"-DDDDDDD", 0x7f)),
                    toggle_bits(ui, &mut self.regs[0x12], ("$4012", b"AAAAAAAA", 0xff)),
                    toggle_bits(ui, &mut self.regs[0x13], ("$4013", b"LLLLLLLL", 0xff)),
                ];

                for (i, c) in changes.iter().enumerate() {
                    if *c {
                        self.write(0x4010 + i as u16, self.regs[0x10 + i]);
                    }
                }
            });

        egui::Window::new("Controls")
            .collapsible(false)
            .anchor(
                egui::Align2::LEFT_TOP,
                rect4.response.rect.left_bottom().to_vec2() + (0.0, 5.0).into(),
            )
            .show(ctx, |ui| {
                if toggle_bits(ui, &mut self.regs[0x15], ("$4015", b"---DNT21", 0x1f)) {
                    self.write(0x4015, self.regs[0x15]);
                }
                if toggle_bits(ui, &mut self.regs[0x17], ("$4017", b"MI------", 0xcf)) {
                    self.write(0x4017, self.regs[0x17]);
                }
            });
    }

    fn name(&self) -> &str {
        "NES APU"
    }
}
