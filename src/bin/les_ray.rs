use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use les::{Bus, Cartridge, Cpu, InputStates};
use raylib::prelude::*;
use std::ffi::CStr;
use std::sync::{Arc, Mutex};

const DISPLAY_SIZE: (i32, i32) = (256, 240);

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
        rl.gui_set_style(
            GuiControl::DEFAULT,
            GuiDefaultProperty::TEXT_SIZE as i32,
            20,
        );
        rl.gui_set_style(
            GuiControl::DEFAULT,
            GuiDefaultProperty::TEXT_SPACING as i32,
            2,
        );

        let render_texture = rl
            .load_render_texture(&thread, DISPLAY_SIZE.0 as u32, DISPLAY_SIZE.1 as u32)
            .unwrap();

        Self {
            rl,
            thread,

            render_texture,
            display_scale,
            draw_fps: false,
            paused: false,

            input0: Self::collect_keyboard_input,
            input1: Self::collect_gamepad_input,
        }
    }

    fn run(&mut self, emu: Arc<Mutex<EmuContext>>) {
        while !self.rl.window_should_close() {
            {
                let mut emu = emu.lock().unwrap();
                let EmuContext { cpu, bus, pause } = &mut *emu;

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
            }

            self.handle_gui_events();
            self.draw_gui();
        }
    }

    fn draw_gui(&mut self) {
        let mut d = self.rl.begin_drawing(&self.thread);

        d.clear_background(Color::GRAY);
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
                Some(CStr::from_bytes_with_nul(b"PAUSED\0").unwrap()),
            )
        } else if self.draw_fps {
            d.draw_fps(5, 5);
        }
    }

    fn handle_gui_events(&mut self) {
        if self.rl.is_key_pressed(KeyboardKey::KEY_EQUAL) {
            self.display_scale = (self.display_scale + 1).min(4);
            self.rl.set_window_size(self.width(), self.height());
        } else if self.rl.is_key_pressed(KeyboardKey::KEY_MINUS) {
            self.display_scale = (self.display_scale - 1).max(1);
            self.rl.set_window_size(self.width(), self.height());
        } else if self.rl.is_key_pressed(KeyboardKey::KEY_F) {
            self.draw_fps = !self.draw_fps;
        } else if self.rl.is_key_pressed(KeyboardKey::KEY_G) {
            std::mem::swap(&mut self.input0, &mut self.input1);
        } else if self.rl.is_key_pressed(KeyboardKey::KEY_LEFT_SHIFT) {
            self.paused = !self.paused;
        }
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
                let EmuContext { bus, cpu, pause } = &mut *emu;

                if *pause {
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
