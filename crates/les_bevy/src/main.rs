use bevy::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use les::{Bus, Cartridge, Cpu};
use std::sync::{Arc, Mutex};

mod pick_file;
mod ui;

struct EmuContext {
    pub cpu: Cpu,
    pub bus: Bus,
    pub pause: bool,
    pub step: bool,
}

type SharedEmuContext = Arc<Mutex<EmuContext>>;

fn main() {
    let emu = {
        let mut bus = Bus::new(Cartridge::empty());
        let mut cpu = Cpu::default();
        bus.reset(&mut cpu);

        Arc::new(Mutex::new(EmuContext {
            cpu,
            bus,
            pause: false,
            step: false,
        }))
    };

    let stream = init_audio(emu.clone());
    stream.play().unwrap();

    App::build()
        .insert_resource(emu)
        .insert_resource(WindowDescriptor {
            title: "LES!".to_owned(),
            vsync: true,
            ..Default::default()
        })
        .add_plugin(bevy::core::CorePlugin::default())
        .add_plugin(bevy::input::InputPlugin::default())
        .add_plugin(bevy::window::WindowPlugin::default())
        .add_plugin(bevy::asset::AssetPlugin::default())
        .add_plugin(bevy::render::RenderPlugin::default())
        .add_plugin(bevy::gilrs::GilrsPlugin::default())
        .add_plugin(bevy::winit::WinitPlugin::default())
        .add_plugin(bevy::wgpu::WgpuPlugin::default())
        .add_plugin(ui::UiPlugin)
        .add_plugin(pick_file::PickFilePlugin)
        .run();
}

fn init_audio(emu: SharedEmuContext) -> cpal::Stream {
    let host = cpal::default_host();
    let device = host.default_output_device().expect("no default device");
    let config = device.default_output_config().unwrap();

    let emu = emu.clone();
    match config.sample_format() {
        cpal::SampleFormat::I16 => run::<i16>(emu, &device, &config.into()),
        cpal::SampleFormat::U16 => run::<u16>(emu, &device, &config.into()),
        cpal::SampleFormat::F32 => run::<f32>(emu, &device, &config.into()),
    }
}

fn run<T>(emu: SharedEmuContext, device: &cpal::Device, config: &cpal::StreamConfig) -> cpal::Stream
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
                    pause,
                    step,
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
                    bus.audio_samples().drain(0..i);
                }
            },
            |err| eprintln!("an error occurred on stream: {}", err),
        )
        .unwrap();

    stream
}