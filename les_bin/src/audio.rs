use bevy::{
    audio::{AddAudioSource, Source},
    prelude::*,
};
use crossbeam_queue::ArrayQueue;

use crate::{ControlEvent, EmuContext, SharedEmuContext};

pub struct AudioRunnerPlugin {
    pub(crate) emu: SharedEmuContext,
}

impl Plugin for AudioRunnerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SharedEmuContextRes(self.emu.clone()))
            .add_audio_source::<EmuAudio>()
            .add_systems(Startup, setup);
    }
}

#[derive(Resource)]
struct SharedEmuContextRes(SharedEmuContext);

#[derive(Asset, TypePath)]
struct EmuAudio {
    emu: SharedEmuContext,
    buf_count: usize,
    sample_rate: u32,
}

struct AudioRunnder {
    emu: SharedEmuContext,
    sample_rate: u32,
    queue_size: usize,
    queue_buf: Box<[i16]>,
    queue: ArrayQueue<i16>,
}

impl AudioRunnder {
    const CHANNELS: usize = 1;

    fn new(emu: SharedEmuContext, queue_size: usize, sample_rate: u32) -> Self {
        AudioRunnder {
            emu,
            sample_rate,
            queue_size,
            queue_buf: vec![0i16; queue_size].into_boxed_slice(),
            queue: ArrayQueue::new(queue_size),
        }
    }
}

impl AudioRunnder {
    fn poll_events(emu: &mut EmuContext) {
        let EmuContext {
            bus,
            cpu,
            pause,
            step,
            cnotrol_events,
        } = &mut *emu;
        while let Ok(ev) = cnotrol_events.try_recv() {
            match ev {
                ControlEvent::LoadCart(data) => {
                    if let Some(cart) = les_nes::Cartridge::load(&data) {
                        bus.load_cart(cart);
                        bus.reset(cpu);
                    }
                }
                ControlEvent::AudioCtrl(states) => bus.set_audio_control(&states),
                ControlEvent::Inputs(p0, p1) => {
                    bus.set_input0(p0);
                    bus.set_input1(p1);
                }
                ControlEvent::Reset => bus.reset(cpu),
                ControlEvent::Pause => *pause = !*pause,
                ControlEvent::Step => {
                    *pause = true;
                    *step = true;
                }
            }
        }
    }
}

impl Iterator for AudioRunnder {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        let mut emu = self.emu.lock().unwrap();
        Self::poll_events(&mut emu);

        let EmuContext {
            bus,
            cpu,
            pause,
            step,
            ..
        } = &mut *emu;

        if self.queue.is_empty() {
            if *pause {
                if *step {
                    bus.exec(cpu);
                    *step = false;
                }

                return Some(0);
            } else {
                let sample_len = self.queue_size / Self::CHANNELS;
                let needed_cycles = bus.resampler().clocks_needed(sample_len);
                let cycles = bus.cycles() + needed_cycles;
                while bus.cycles() < cycles {
                    bus.exec(cpu);
                }

                bus.resampler().end_frame();
                bus.resampler()
                    .read_samples(&mut self.queue_buf[..sample_len]);
                self.queue_buf[..sample_len].iter().for_each(|s| {
                    let _ = self.queue.push(*s);
                });
            }
        }

        self.queue.pop()
    }
}
impl Source for AudioRunnder {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        Self::CHANNELS as _
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

impl Decodable for EmuAudio {
    type DecoderItem = <AudioRunnder as Iterator>::Item;

    type Decoder = AudioRunnder;

    fn decoder(&self) -> Self::Decoder {
        AudioRunnder::new(self.emu.clone(), self.buf_count, self.sample_rate)
    }
}

fn setup(
    mut assets: ResMut<Assets<EmuAudio>>,
    emu: Res<SharedEmuContextRes>,
    mut commands: Commands,
) {
    const SAMPLE_RATE: u32 = 44_100;
    const BUF_COUNT: usize = 256;

    emu.0
        .lock()
        .unwrap()
        .bus
        .resampler()
        .set_rates(SAMPLE_RATE as _);

    commands.spawn(AudioPlayer(assets.add(EmuAudio {
        emu: emu.0.clone(),
        buf_count: BUF_COUNT,
        sample_rate: SAMPLE_RATE,
    })));
}
