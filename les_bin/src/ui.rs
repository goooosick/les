use super::{ControlEvent, ControlSender, EmuContext, SharedEmuContext};
use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::texture::ImageSampler,
    tasks::IoTaskPool,
};
use bevy_egui::{
    egui::{self, TextureId},
    EguiContext,
};
use les_nes::{cpu::CpuStatus, InputStates};

struct PickRom;

pub struct UiPlugin(pub(crate) ControlSender);

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(bevy_egui::EguiPlugin)
            .insert_resource(UiData {
                scale: 2,
                debug_scales: [1.0; 4],
                apu_ctrl: [true; 5],
                ..Default::default()
            })
            .insert_resource(self.0.clone())
            .init_resource::<Option<Gamepad>>()
            .add_event::<PickRom>()
            .add_startup_system(alloc_textures)
            .add_system(ui)
            .add_system(pick_rom)
            .add_system(sync_emu_status)
            .add_system(gamepad_connection)
            .add_system(handle_inputs);
    }
}

struct PpuTexture {
    id: TextureId,
    size: egui::Vec2,
    name: &'static str,
    handle: Handle<Image>,
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
    swap_input: bool,
}

fn ui(
    mut egui_context: ResMut<EguiContext>,
    infos: Res<PpuTextures>,
    mut ui_data: ResMut<UiData>,
    diagnostics: Res<Diagnostics>,
    control_sender: Res<ControlSender>,
    mut pick_rom: EventWriter<PickRom>,
) {
    use egui::{menu, Slider};

    let ctx = egui_context.ctx_mut();

    egui::TopBottomPanel::top("").show(ctx, |ui| {
        menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("open").clicked() {
                    pick_rom.send(PickRom);
                }
            });
            ui.menu_button("Debug", |ui| {
                ui.checkbox(&mut ui_data.debug, "debug_windows");
            });
            ui.menu_button("Layout", |ui| {
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
                    let mut changed = false;
                    for (value, name) in apu_ctrl
                        .iter_mut()
                        .zip(["Pulse1", "Pulse2", "Triangle", "Noise", "DMC"].into_iter())
                    {
                        changed |= ui.checkbox(value, name).changed();
                    }
                    if changed {
                        let _ = control_sender.send(ControlEvent::AudioCtrl(*apu_ctrl));
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
                            let _ = control_sender.send(ControlEvent::Reset);
                        }
                        if ui.button("STEP").clicked() {
                            let _ = control_sender.send(ControlEvent::Step);
                        }
                        if ui.button("CONTINUE").clicked() {
                            let _ = control_sender.send(ControlEvent::Pause);
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

        egui::Window::new(format!(
            "les-{:3.02}",
            diagnostics
                .get(FrameTimeDiagnosticsPlugin::FPS)
                .unwrap()
                .average()
                .unwrap_or_default()
        ))
        .id(egui::Id::new("window"))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.image(infos[0].id, infos[0].size * ui_data.scale as f32);
        });
    });
}

fn alloc_textures(
    mut command: Commands,
    mut assets: ResMut<Assets<Image>>,
    mut egui_context: ResMut<EguiContext>,
) {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    const TEXTURE_INFOS: [((usize, usize), &str); 5] = [
        ((256, 240), ""),
        ((256, 128), "Pattern"),
        ((256, 240), "Nametable"),
        ((256, 32), "Palettes"),
        ((256, 16), "Sprites"),
    ];

    let mut images = vec![];

    TEXTURE_INFOS.into_iter().for_each(|(size, name)| {
        let mut image = Image::new(
            Extent3d {
                width: size.0 as _,
                height: size.1 as _,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![255u8; size.0 * size.1 * 4],
            TextureFormat::Rgba8UnormSrgb,
        );
        image.sampler_descriptor = ImageSampler::nearest();

        let handle = assets.add(image);
        let id = egui_context.add_image(handle.clone_weak());

        images.push(PpuTexture {
            id,
            size: (size.0 as f32, size.1 as f32).into(),
            name,
            handle,
        });
    });

    command.insert_resource(images);
}

fn sync_emu_status(
    mut textures: ResMut<Assets<Image>>,
    infos: Res<PpuTextures>,
    emu: Res<SharedEmuContext>,
    mut ui_data: ResMut<UiData>,
) {
    let mut emu = emu.lock().unwrap();
    let EmuContext { cpu, bus, .. } = &mut *emu;

    fn as_chunks_mut(slice: &mut [u8]) -> &mut [[u8; 4]] {
        assert_eq!(slice.len() % 4, 0);
        unsafe { std::slice::from_raw_parts_mut(slice.as_mut_ptr().cast(), slice.len() / 4) }
    }

    let ppu = bus.ppu();

    if let Some(tex) = textures.get_mut(&infos[0].handle) {
        ppu.render_display(as_chunks_mut(tex.data.as_mut()));
    }
    if ui_data.debug {
        let cart = bus.cart();

        if let Some(tex) = textures.get_mut(&infos[1].handle) {
            ppu.render_pattern_table(cart, as_chunks_mut(tex.data.as_mut()), ui_data.pat_index);
        }
        if let Some(tex) = textures.get_mut(&infos[2].handle) {
            ppu.render_name_table(cart, as_chunks_mut(tex.data.as_mut()), ui_data.nm_index);
        }
        if let Some(tex) = textures.get_mut(&infos[3].handle) {
            ppu.render_palettes(as_chunks_mut(tex.data.as_mut()));
        }
        if let Some(tex) = textures.get_mut(&infos[4].handle) {
            ppu.render_sprites(cart, as_chunks_mut(tex.data.as_mut()));
        }
    }

    ui_data.nes_status = NesStatus {
        cpu_status: Some(cpu.status()),
        ppu_timing: bus.ppu().timing(),
        ppu_frames: bus.ppu().frame_count(),
        cycles: bus.cycles(),
    };
}

fn handle_inputs(
    input: Res<Input<KeyCode>>,
    gamepad: Res<Option<Gamepad>>,
    button_inputs: Res<Input<GamepadButton>>,
    control_sender: Res<ControlSender>,
    mut ui_data: ResMut<UiData>,
) {
    if input.just_pressed(KeyCode::R) {
        let _ = control_sender.send(ControlEvent::Reset);
    } else if input.pressed(KeyCode::S) {
        let _ = control_sender.send(ControlEvent::Step);
    } else if input.just_pressed(KeyCode::LShift) {
        let _ = control_sender.send(ControlEvent::Pause);
    }

    let game_inputs = collect_inputs(&input, &gamepad, &button_inputs, ui_data.swap_input);
    let _ = control_sender.send(ControlEvent::Inputs(game_inputs.0, game_inputs.1));

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
            GamepadEvent {
                gamepad: g,
                event_type: GamepadEventType::Connected,
            } => {
                if gamepad.is_none() {
                    gamepad.replace(*g);
                }
            }
            GamepadEvent {
                gamepad: g,
                event_type: GamepadEventType::Disconnected,
            } => {
                if (*gamepad).as_ref() == Some(g) {
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
    let input0 = les_nes::InputStates {
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
        gamepad.map_or(Default::default(), |g| les_nes::InputStates {
            a: bis.pressed(GamepadButton::new(g, GamepadButtonType::South)),
            b: bis.pressed(GamepadButton::new(g, GamepadButtonType::East)),
            select: bis.pressed(GamepadButton::new(g, GamepadButtonType::Select)),
            start: bis.pressed(GamepadButton::new(g, GamepadButtonType::Start)),
            up: bis.pressed(GamepadButton::new(g, GamepadButtonType::DPadUp)),
            down: bis.pressed(GamepadButton::new(g, GamepadButtonType::DPadDown)),
            left: bis.pressed(GamepadButton::new(g, GamepadButtonType::DPadLeft)),
            right: bis.pressed(GamepadButton::new(g, GamepadButtonType::DPadRight)),
        })
    };

    if swap {
        (input1, input0)
    } else {
        (input0, input1)
    }
}

fn pick_rom(sender: Res<ControlSender>, mut events: EventReader<PickRom>) {
    if events.iter().next().is_some() {
        let task_pool = IoTaskPool::get();
        let sender = sender.clone();
        task_pool
            .spawn(async move {
                if let Some(handle) = rfd::AsyncFileDialog::new().pick_file().await {
                    let _ = sender.send(ControlEvent::LoadCart(handle.read().await));
                }
            })
            .detach();
    }
}
