use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use les::{Bus, Cartridge, Cpu, InputStates};
use raylib::prelude::*;
use std::ffi::CStr;
use std::sync::{Arc, Mutex};

const DISPLAY_SIZE: (i32, i32) = (256, 240);
const PATTERN_SIZE: (usize, usize) = (256, 128);
const NAMETABLE_SIZE: (usize, usize) = (256, 240);
const PALETTES_SIZE: (usize, usize) = (256, 32);
const SPRITES_SIZE: (usize, usize) = (256, 16);

const DEBUG_W0: f32 = 300.0;
const DEBUG_W1: f32 = 535.0;
const DEBUG_PAD_W: f32 = 10.0;
const DEBUG_BOUNDS: (f32, f32) = (DEBUG_W0 + DEBUG_W1 + 3.0 * DEBUG_PAD_W, 590.0);

macro_rules! cstr {
    ($fmt: expr) => { cstr!($fmt,) };
    ($fmt: expr, $($args: tt) *) => {
        Some(CStr::from_bytes_with_nul(format!(concat!($fmt, "\0"), $($args) *).as_bytes()).unwrap())
    };
}

mod draw_gui;

fn main() {
    raylib::logging::set_trace_log(TraceLogType::LOG_WARNING);

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

    let mut gui = GuiContext::new(2);

    let stream = init_audio(emu.clone());
    stream.play().unwrap();

    gui.run(emu);
}

struct EmuContext {
    cpu: Cpu,
    bus: Bus,
    pause: bool,
    step: bool,
}

type InputFunc = fn(&RaylibHandle) -> InputStates;

struct GuiContext {
    rl: RaylibHandle,
    thread: RaylibThread,

    render_texture: RenderTexture2D,
    display_scale: i32,
    draw_fps: bool,
    paused: bool,

    input0: InputFunc,
    input1: InputFunc,

    debug: bool,
    pal_index: usize,
    audio_ctrl: [bool; 5],
    debug_textures: Vec<(RenderTexture2D, Vec<[u8; 3]>)>,
}

impl GuiContext {
    fn new(display_scale: i32) -> Self {
        let width = DISPLAY_SIZE.0 * display_scale;
        let height = DISPLAY_SIZE.1 * display_scale;

        let (mut rl, thread) = raylib::init()
            .size(width, height)
            .title("LES")
            .resizable()
            .vsync()
            .build();
        rl.gui_load_style(cstr!("style.rgs"));

        let render_texture = rl
            .load_render_texture(&thread, DISPLAY_SIZE.0 as u32, DISPLAY_SIZE.1 as u32)
            .unwrap();

        let mut debug_textures = vec![];
        for size in [PATTERN_SIZE, NAMETABLE_SIZE, PALETTES_SIZE, SPRITES_SIZE] {
            let n = if size == NAMETABLE_SIZE { 4 } else { 1 };
            for _ in 0..n {
                debug_textures.push((
                    rl.load_render_texture(&thread, size.0 as u32, size.1 as u32)
                        .unwrap(),
                    vec![[0u8; 3]; size.0 * size.1],
                ));
            }
        }

        Self {
            rl,
            thread,

            render_texture,
            display_scale,
            draw_fps: false,
            paused: false,

            input0: Self::collect_keyboard_input,
            input1: Self::collect_gamepad_input,

            debug: false,
            pal_index: 0,
            audio_ctrl: [true; 5],
            debug_textures,
        }
    }

    fn run(&mut self, emu: Arc<Mutex<EmuContext>>) {
        while !self.rl.window_should_close() {
            {
                let mut emu = emu.lock().unwrap();
                let EmuContext {
                    cpu, bus, pause, ..
                } = &mut *emu;

                if self.rl.is_key_pressed(KeyboardKey::KEY_R) {
                    bus.reset(cpu);
                } else if self.rl.is_file_dropped() {
                    let cart = Cartridge::load(&self.rl.get_dropped_files()[0]).unwrap();
                    bus.load_cart(cart);
                    bus.reset(cpu);

                    self.rl.clear_dropped_files();
                } else {
                    bus.set_input0((self.input0)(&self.rl));
                    bus.set_input1((self.input1)(&self.rl));

                    self.render_texture.update_texture(
                        bus.ppu()
                            .display_buf()
                            .chunks_exact(3)
                            .flat_map(|c| [c[0], c[1], c[2], 255])
                            .collect::<Vec<u8>>()
                            .as_ref(),
                    );
                }

                *pause = self.paused;

                self.draw_gui(emu);
            }

            self.handle_gui_events();
        }
    }

    fn handle_gui_events(&mut self) {
        if self.rl.is_key_pressed(KeyboardKey::KEY_EQUAL) {
            self.display_scale = (self.display_scale + 1).min(4);
            self.update_window_size();
        } else if self.rl.is_key_pressed(KeyboardKey::KEY_MINUS) {
            self.display_scale = (self.display_scale - 1).max(1);
            self.update_window_size();
        } else if self.rl.is_key_pressed(KeyboardKey::KEY_F) {
            self.draw_fps = !self.draw_fps;
        } else if self.rl.is_key_pressed(KeyboardKey::KEY_G) {
            std::mem::swap(&mut self.input0, &mut self.input1);
        } else if self.rl.is_key_pressed(KeyboardKey::KEY_LEFT_SHIFT) {
            self.paused = !self.paused;
        } else if self.rl.is_key_pressed(KeyboardKey::KEY_TAB) {
            self.debug = !self.debug;
            self.update_window_size();
        }
    }

    fn update_window_size(&mut self) {
        let mut w = self.width();
        let mut h = self.height();

        if self.debug {
            w = w + DEBUG_BOUNDS.0.ceil() as i32;
            h = h.max(DEBUG_BOUNDS.1 as i32);
        }

        self.rl.set_window_size(w, h);
    }

    fn width(&self) -> i32 {
        self.display_scale * DISPLAY_SIZE.0
    }

    fn height(&self) -> i32 {
        self.display_scale * DISPLAY_SIZE.1
    }

    fn collect_keyboard_input(rl: &RaylibHandle) -> InputStates {
        InputStates {
            a: rl.is_key_down(KeyboardKey::KEY_Z),
            b: rl.is_key_down(KeyboardKey::KEY_X),
            select: rl.is_key_down(KeyboardKey::KEY_C),
            start: rl.is_key_down(KeyboardKey::KEY_V),
            up: rl.is_key_down(KeyboardKey::KEY_UP),
            down: rl.is_key_down(KeyboardKey::KEY_DOWN),
            left: rl.is_key_down(KeyboardKey::KEY_LEFT),
            right: rl.is_key_down(KeyboardKey::KEY_RIGHT),
        }
    }

    fn collect_gamepad_input(rl: &RaylibHandle) -> InputStates {
        use std::f32::consts::PI;
        use GamepadAxis::*;
        use GamepadButton::*;

        const PAD: GamepadNumber = GamepadNumber::GAMEPAD_PLAYER1;

        if rl.is_gamepad_available(PAD) {
            let index = {
                let y = rl.get_gamepad_axis_movement(PAD, GAMEPAD_AXIS_LEFT_Y);
                let x = rl.get_gamepad_axis_movement(PAD, GAMEPAD_AXIS_LEFT_X);

                if y != 0.0 || x != 0.0 {
                    let mut rad = y.atan2(x);
                    if rad < 0.0 {
                        rad += 2.0 * PI;
                    }

                    let sec = 360.0 / 4.0;
                    let half_sec = sec / 2.0;
                    let angle = rad.to_degrees();

                    ((angle + half_sec) / sec).floor() as u32 % 4
                } else {
                    4
                }
            };

            InputStates {
                a: rl.is_gamepad_button_down(PAD, GAMEPAD_BUTTON_RIGHT_FACE_DOWN)
                    | rl.is_gamepad_button_down(PAD, GAMEPAD_BUTTON_RIGHT_TRIGGER_2),
                b: rl.is_gamepad_button_down(PAD, GAMEPAD_BUTTON_RIGHT_FACE_RIGHT)
                    | rl.is_gamepad_button_down(PAD, GAMEPAD_BUTTON_LEFT_TRIGGER_2),
                select: rl.is_gamepad_button_down(PAD, GAMEPAD_BUTTON_MIDDLE_LEFT),
                start: rl.is_gamepad_button_down(PAD, GAMEPAD_BUTTON_MIDDLE_RIGHT),
                up: index == 3 || rl.is_gamepad_button_down(PAD, GAMEPAD_BUTTON_LEFT_FACE_UP),
                down: index == 1 || rl.is_gamepad_button_down(PAD, GAMEPAD_BUTTON_LEFT_FACE_DOWN),
                left: index == 2 || rl.is_gamepad_button_down(PAD, GAMEPAD_BUTTON_LEFT_FACE_LEFT),
                right: index == 0 || rl.is_gamepad_button_down(PAD, GAMEPAD_BUTTON_LEFT_FACE_RIGHT),
            }
        } else {
            Default::default()
        }
    }
}

fn init_audio(emu: Arc<Mutex<EmuContext>>) -> cpal::Stream {
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
