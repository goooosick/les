use super::{pick_file::*, EmuContext, SharedEmuContext};
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, TextureId},
    EguiContext, EguiPlugin,
};
use les::{cpu::CpuStatus, Cartridge, InputStates};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(EguiPlugin)
            .insert_resource(UiData {
                scale: 2,
                debug_scales: [1.0; 4],
                apu_ctrl: [true; 5],
                ..Default::default()
            })
            .init_resource::<Option<Gamepad>>()
            .add_startup_system(alloc_textures.system())
            .add_system(ui.system())
            .add_system(load_rom_event.system())
            .add_system(gamepad_connection.system())
            .add_system(update.system());
    }
}

struct PpuTexture {
    id: TextureId,
    size: egui::Vec2,
    name: &'static str,
    handle: Handle<Texture>,
}

type PpuTextures = Vec<PpuTexture>;

#[derive(Default)]
struct NesStatus {
    cpu_status: Option<CpuStatus>,
    ppu_timing: (usize, usize),
    ppu_frames: usize,
    cycles: usize,
}

#[derive(Default)]
struct UiData {
    debug: bool,
    scale: usize,
    debug_scales: [f32; 4],
    apu_ctrl: [bool; 5],
    pat_index: usize,
    nm_index: usize,
    nes_status: NesStatus,
    reset: Option<()>,
    step: Option<()>,
    r#continue: Option<()>,
    swap_input: bool,
}

fn ui(
    egui_context: ResMut<EguiContext>,
    infos: Res<PpuTextures>,
    mut ui_data: ResMut<UiData>,
    mut file_events: EventWriter<RequestFile>,
) {
    use egui::{menu, Slider};

    let ctx = egui_context.ctx();

    egui::TopBottomPanel::top("").show(ctx, |ui| {
        menu::bar(ui, |ui| {
            menu::menu(ui, "File", |ui| {
                if ui.button("open").clicked() {
                    file_events.send(RequestFile);
                }
            });
            menu::menu(ui, "Debug", |ui| {
                ui.checkbox(&mut ui_data.debug, "debug_windows");
            });
            menu::menu(ui, "Layout", |ui| {
                if ui.button("reset").clicked() {
                    ctx.memory().reset_areas();
                }
            });
        });
    });

    egui::CentralPanel::default().show(ctx, |_ui| {
        if ui_data.debug {
            let UiData {
                pat_index,
                nm_index,
                debug_scales,
                apu_ctrl,
                ..
            } = &mut *ui_data;

            for (index, (tex, dscale)) in infos
                .iter()
                .skip(1)
                .zip(debug_scales.iter_mut())
                .enumerate()
            {
                egui::Window::new(tex.name)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.add(Slider::new(dscale, 1.0..=4.0).text("scale"));
                        ui.image(tex.id, tex.size * *dscale);

                        if index == 0 {
                            ui.add(Slider::new(pat_index, 0..=7).text("index"));
                        } else if index == 1 {
                            ui.add(Slider::new(nm_index, 0..=3).text("index"));
                        }
                    });
            }

            egui::Window::new("APU").resizable(false).show(ctx, |ui| {
                ui.vertical(|ui| {
                    for (value, name) in apu_ctrl
                        .iter_mut()
                        .zip(["Pulse1", "Pulse2", "Triangle", "Noise", "DMC"].iter())
                    {
                        ui.checkbox(value, name);
                    }
                });
            });

            egui::Window::new("CPU").resizable(false).show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.vertical(|ui| {
                        if let Some(s) = &ui_data.nes_status.cpu_status {
                            ui.label(format!("A: {:02X} X: {:02X} Y: {:02X}", s.a, s.x, s.y));
                            ui.label(format!("PC: {:04X} SP: {:04X}", s.pc, s.sp));
                            ui.label(format!("P: {:?}  {:02X}", s.p, s.p.to_u8()));
                            ui.label(format!("CYCLES: {}", ui_data.nes_status.cycles));
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("RESET").clicked() {
                            ui_data.reset = Some(());
                        }
                        if ui.button("STEP").clicked() {
                            ui_data.step = Some(());
                        }
                        if ui.button("CONTINUE").clicked() {
                            ui_data.r#continue = Some(());
                        }
                    });
                });
            });

            egui::Window::new("PPU").resizable(false).show(ctx, |ui| {
                let s = &ui_data.nes_status;
                ui.label(format!(
                    "TIMING: ({:03}, {:03})",
                    s.ppu_timing.0, s.ppu_timing.1
                ));
                ui.label(format!("FRAMES: {:15}", s.ppu_frames));
            });
        }

        egui::Window::new("les")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.image(infos[0].id, infos[0].size * ui_data.scale as f32);
            });
    });
}

fn alloc_textures(
    mut command: Commands,
    mut assets: ResMut<Assets<Texture>>,
    mut egui_context: ResMut<EguiContext>,
) {
    use bevy::render::texture::{Extent3d, TextureDimension, TextureFormat};

    const TEXTURE_INFOS: [((usize, usize), &'static str); 5] = [
        ((256, 240), ""),
        ((256, 128), "Pattern"),
        ((256, 240), "Nametable"),
        ((256, 32), "Palettes"),
        ((256, 16), "Sprites"),
    ];

    let mut textures = vec![];

    for (i, (size, name)) in TEXTURE_INFOS.into_iter().enumerate() {
        let handle = assets.add(Texture::new(
            Extent3d {
                width: size.0 as _,
                height: size.1 as _,
                depth: 1,
            },
            TextureDimension::D2,
            vec![0u8; size.0 * size.1 * 4],
            TextureFormat::Rgba8UnormSrgb,
        ));

        egui_context.set_egui_texture(i as _, handle.as_weak());
        textures.push(PpuTexture {
            id: egui::TextureId::User(i as _),
            size: (size.0 as f32, size.1 as f32).into(),
            name,
            handle,
        });
    }

    command.insert_resource(textures);
}

fn update(
    input: Res<Input<KeyCode>>,
    gamepad: Res<Option<Gamepad>>,
    button_inputs: Res<Input<GamepadButton>>,
    mut textures: ResMut<Assets<Texture>>,
    infos: Res<PpuTextures>,
    emu: Res<SharedEmuContext>,
    mut ui_data: ResMut<UiData>,
) {
    {
        let mut emu = emu.lock().unwrap();
        let EmuContext {
            cpu,
            bus,
            pause,
            step,
        } = &mut *emu;

        {
            fn as_chunks_mut(slice: &mut [u8]) -> &mut [[u8; 4]] {
                assert_eq!(slice.len() % 4, 0);
                unsafe {
                    std::slice::from_raw_parts_mut(slice.as_mut_ptr().cast(), slice.len() / 4)
                }
            }

            let ppu = bus.ppu();

            if let Some(tex) = textures.get_mut(infos[0].handle.clone()) {
                ppu.render_display(as_chunks_mut(tex.data.as_mut()));
            }
            if ui_data.debug {
                let cart = bus.cart();

                if let Some(tex) = textures.get_mut(infos[1].handle.clone()) {
                    ppu.render_pattern_table(
                        cart,
                        as_chunks_mut(tex.data.as_mut()),
                        ui_data.pat_index,
                    );
                }
                if let Some(tex) = textures.get_mut(infos[2].handle.clone()) {
                    ppu.render_name_table(cart, as_chunks_mut(tex.data.as_mut()), ui_data.nm_index);
                }
                if let Some(tex) = textures.get_mut(infos[3].handle.clone()) {
                    ppu.render_palettes(as_chunks_mut(tex.data.as_mut()));
                }
                if let Some(tex) = textures.get_mut(infos[4].handle.clone()) {
                    ppu.render_sprites(cart, as_chunks_mut(tex.data.as_mut()));
                }
            }
        }

        if input.just_pressed(KeyCode::R) {
            bus.reset(cpu);
        } else if input.pressed(KeyCode::S) {
            *pause = true;
            *step = true;
        } else if input.just_pressed(KeyCode::LShift) {
            *pause = !*pause;
        }

        let game_inputs = collect_inputs(&input, &gamepad, &button_inputs, ui_data.swap_input);
        bus.set_input0(game_inputs.0);
        bus.set_input1(game_inputs.1);
        bus.set_audio_control(&ui_data.apu_ctrl);

        ui_data.nes_status = NesStatus {
            cpu_status: Some(cpu.status()),
            ppu_timing: bus.ppu().timing(),
            ppu_frames: bus.ppu().frame_count(),
            cycles: bus.cycles(),
        };

        if ui_data.reset.is_some() {
            bus.reset(cpu);
            ui_data.reset.take();
        }
        if ui_data.step.is_some() {
            *pause = true;
            *step = true;
            ui_data.step.take();
        }
        if ui_data.r#continue.is_some() {
            *pause = false;
            ui_data.r#continue.take();
        }
    }

    if input.just_pressed(KeyCode::Equals) {
        ui_data.scale = (ui_data.scale + 1).min(4);
    } else if input.just_pressed(KeyCode::Minus) {
        ui_data.scale = (ui_data.scale - 1).max(1);
    } else if input.just_pressed(KeyCode::Tab) {
        ui_data.debug = !ui_data.debug;
    } else if input.just_pressed(KeyCode::G) {
        ui_data.swap_input = !ui_data.swap_input;
    }
}

fn gamepad_connection(
    mut gamepad: ResMut<Option<Gamepad>>,
    mut gamepad_event: EventReader<GamepadEvent>,
) {
    for event in gamepad_event.iter() {
        match &event {
            GamepadEvent(g, GamepadEventType::Connected) => {
                if gamepad.is_none() {
                    gamepad.replace(*g);
                }
            }
            GamepadEvent(g, GamepadEventType::Disconnected) => {
                if gamepad.as_ref() == Some(g) {
                    gamepad.take();
                }
            }
            _ => (),
        }
    }
}

fn collect_inputs(
    input: &Res<Input<KeyCode>>,
    gamepad: &Res<Option<Gamepad>>,
    button_inputs: &Res<Input<GamepadButton>>,
    swap: bool,
) -> (InputStates, InputStates) {
    let input0 = les::InputStates {
        a: input.pressed(KeyCode::Z),
        b: input.pressed(KeyCode::X),
        select: input.pressed(KeyCode::C),
        start: input.pressed(KeyCode::V),
        up: input.pressed(KeyCode::Up),
        down: input.pressed(KeyCode::Down),
        left: input.pressed(KeyCode::Left),
        right: input.pressed(KeyCode::Right),
    };
    let input1 = {
        let bis = button_inputs;
        gamepad.map_or(Default::default(), |g| les::InputStates {
            a: bis.pressed(GamepadButton(g, GamepadButtonType::South)),
            b: bis.pressed(GamepadButton(g, GamepadButtonType::East)),
            select: bis.pressed(GamepadButton(g, GamepadButtonType::Select)),
            start: bis.pressed(GamepadButton(g, GamepadButtonType::Start)),
            up: bis.pressed(GamepadButton(g, GamepadButtonType::DPadUp)),
            down: bis.pressed(GamepadButton(g, GamepadButtonType::DPadDown)),
            left: bis.pressed(GamepadButton(g, GamepadButtonType::DPadLeft)),
            right: bis.pressed(GamepadButton(g, GamepadButtonType::DPadRight)),
        })
    };

    if swap {
        (input1, input0)
    } else {
        (input0, input1)
    }
}

fn load_rom_event(emu: Res<SharedEmuContext>, mut events: EventReader<SelectFile>) {
    for file in events.iter() {
        if let Some(cart) = Cartridge::load(file.0.as_path()) {
            let mut emu = emu.lock().unwrap();
            let EmuContext { cpu, bus, .. } = &mut *emu;

            bus.load_cart(cart);
            bus.reset(cpu);

            break;
        }
    }
}
