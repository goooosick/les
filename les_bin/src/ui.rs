use super::{ControlEvent, ControlSender, EmuContext, SharedEmuContext};
use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    image::ImageSampler,
    prelude::*,
};
use bevy_egui::{
    egui::{self, load::SizedTexture, TextureId},
    EguiContexts, EguiPrimaryContextPass,
};
use leafwing_input_manager::prelude::*;
use les_nes::{cpu::CpuStatus, InputStates};

pub struct UiPlugin {
    pub(crate) emu: SharedEmuContext,
    pub(crate) control_sender: ControlSender,
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_egui::EguiPlugin::default())
            .add_plugins(InputManagerPlugin::<InputAction>::default())
            .insert_resource(UiData {
                scale: 2.0,
                apu_ctrl: [true; 5],
                ..Default::default()
            })
            .insert_resource(SharedEmuContextRes(self.emu.clone()))
            .insert_resource(ControlSenderRes(self.control_sender.clone()))
            .add_message::<PickRom>()
            .add_systems(Startup, setup_camera_system)
            .add_systems(Startup, alloc_textures)
            .add_systems(Startup, spawn_players)
            .add_systems(EguiPrimaryContextPass, ui)
            .add_systems(Update, pick_rom)
            .add_systems(Update, handle_inputs)
            .add_systems(FixedUpdate, sync_emu_status)
            .insert_resource(Time::<Fixed>::from_seconds(59.0f64.recip()));
    }
}

struct PpuTexture {
    id: TextureId,
    size: egui::Vec2,
    name: &'static str,
    handle: Handle<Image>,
}

#[derive(Resource)]
struct PpuTextures(Vec<PpuTexture>);

#[derive(Default)]
struct NesStatus {
    cpu_status: Option<CpuStatus>,
    ppu_timing: (usize, usize),
    ppu_frames: usize,
    cycles: usize,
}

#[derive(Default, Resource)]
struct UiData {
    debug: bool,
    scale: f32,
    apu_ctrl: [bool; 5],
    pat_index: usize,
    nm_index: usize,
    nes_status: NesStatus,
    swap_input: bool,
}

#[derive(Resource)]
struct ControlSenderRes(ControlSender);

#[derive(Resource)]
struct SharedEmuContextRes(SharedEmuContext);

#[derive(Message)]
struct PickRom;

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
enum InputAction {
    A,
    B,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
}

#[derive(Component)]
struct Player1;

#[derive(Component)]
struct Player2;

fn setup_camera_system(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn ui(
    mut egui_context: EguiContexts,
    infos: Res<PpuTextures>,
    mut ui_data: ResMut<UiData>,
    diagnostics: Res<DiagnosticsStore>,
    control_sender: Res<ControlSenderRes>,
    mut pick_rom: MessageWriter<PickRom>,
) -> Result {
    let ctx = egui_context.ctx_mut()?;
    let infos = &infos.0;
    let control_sender = &control_sender.0;

    egui::TopBottomPanel::top("").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("open").clicked() {
                    pick_rom.write(PickRom);
                }
            });
            ui.menu_button("Debug", |ui| {
                ui.checkbox(&mut ui_data.debug, "debug panels");
                ui.checkbox(&mut ui_data.swap_input, "swap player");
            });
            ui.menu_button("Layout", |ui| {
                if ui.button("reset").clicked() {
                    ctx.memory_mut(|mem| mem.reset_areas());
                }
            });
        });
    });

    egui::CentralPanel::default().show(ctx, |_ui| {
        if ui_data.debug {
            let UiData {
                pat_index,
                nm_index,
                apu_ctrl,
                ..
            } = &mut *ui_data;

            for (index, tex) in infos.iter().skip(1).enumerate() {
                egui::Window::new(tex.name)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.image(SizedTexture::new(tex.id, tex.size));

                        if index == 0 {
                            ui.add(egui::Slider::new(pat_index, 0..=7).text("index"));
                        } else if index == 1 {
                            ui.add(egui::Slider::new(nm_index, 0..=3).text("index"));
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
                .get(&FrameTimeDiagnosticsPlugin::FPS)
                .unwrap()
                .average()
                .unwrap_or_default()
        ))
        .id(egui::Id::new("window"))
        .collapsible(false)
        .show(ctx, |ui| {
            ui.image(SizedTexture::new(
                infos[0].id,
                infos[0].size * ui_data.scale,
            ));
            ui.add(egui::Slider::new(&mut ui_data.scale, 1.0..=3.0));
        });
    });

    Ok(())
}

fn alloc_textures(
    mut command: Commands,
    mut assets: ResMut<Assets<Image>>,
    mut egui_context: EguiContexts,
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
            bevy::asset::RenderAssetUsages::all(),
        );
        image.sampler = ImageSampler::nearest();

        let handle = assets.add(image);
        let id = egui_context.add_image(bevy_egui::EguiTextureHandle::Strong(handle.clone()));

        images.push(PpuTexture {
            id,
            size: (size.0 as f32, size.1 as f32).into(),
            name,
            handle,
        });
    });

    command.insert_resource(PpuTextures(images));
}

fn sync_emu_status(
    mut textures: ResMut<Assets<Image>>,
    infos: Res<PpuTextures>,
    emu: Res<SharedEmuContextRes>,
    mut ui_data: ResMut<UiData>,
) {
    let mut emu = emu.0.lock().unwrap();
    let EmuContext { cpu, bus, .. } = &mut *emu;

    fn image_as_mut(image: Option<&mut Image>) -> &mut [[u8; 4]] {
        image
            .unwrap()
            .data
            .as_mut()
            .unwrap()
            .as_mut_slice()
            .as_chunks_mut::<4>()
            .0
    }

    let ppu = bus.ppu();
    let infos = &infos.0;

    ppu.render_display(image_as_mut(textures.get_mut(&infos[0].handle)));
    if ui_data.debug {
        let cart = bus.cart();

        ppu.render_pattern_table(
            cart,
            image_as_mut(textures.get_mut(&infos[1].handle)),
            ui_data.pat_index,
        );
        ppu.render_name_table(
            cart,
            image_as_mut(textures.get_mut(&infos[2].handle)),
            ui_data.nm_index,
        );
        ppu.render_palettes(image_as_mut(textures.get_mut(&infos[3].handle)));
        ppu.render_sprites(cart, image_as_mut(textures.get_mut(&infos[4].handle)));
    }

    ui_data.nes_status = NesStatus {
        cpu_status: Some(cpu.status()),
        ppu_timing: bus.ppu().timing(),
        ppu_frames: bus.ppu().frame_count(),
        cycles: bus.cycles(),
    };
}

fn spawn_players(mut commands: Commands) {
    commands
        .spawn(InputMap::new([
            (InputAction::A, KeyCode::KeyZ),
            (InputAction::B, KeyCode::KeyX),
            (InputAction::Select, KeyCode::KeyC),
            (InputAction::Start, KeyCode::KeyV),
            (InputAction::Up, KeyCode::ArrowUp),
            (InputAction::Down, KeyCode::ArrowDown),
            (InputAction::Left, KeyCode::ArrowLeft),
            (InputAction::Right, KeyCode::ArrowRight),
        ]))
        .insert(Player1);
    commands
        .spawn(InputMap::new([
            (InputAction::A, GamepadButton::South),
            (InputAction::B, GamepadButton::East),
            (InputAction::Select, GamepadButton::Select),
            (InputAction::Start, GamepadButton::Start),
            (InputAction::Up, GamepadButton::DPadUp),
            (InputAction::Down, GamepadButton::DPadDown),
            (InputAction::Left, GamepadButton::DPadLeft),
            (InputAction::Right, GamepadButton::DPadRight),
        ]))
        .insert(Player2);
}

fn handle_inputs(
    query_p1: Query<&ActionState<InputAction>, With<Player1>>,
    query_p2: Query<&ActionState<InputAction>, With<Player2>>,
    input: Res<ButtonInput<KeyCode>>,
    control_sender: Res<ControlSenderRes>,
    ui_data: Res<UiData>,
) -> Result {
    let control_sender = &control_sender.0;

    if input.just_pressed(KeyCode::KeyR) {
        let _ = control_sender.send(ControlEvent::Reset);
    } else if input.pressed(KeyCode::KeyS) {
        let _ = control_sender.send(ControlEvent::Step);
    } else if input.just_pressed(KeyCode::ShiftLeft) {
        let _ = control_sender.send(ControlEvent::Pause);
    }

    let states_p1 = action_to_states(query_p1.single()?);
    let states_p2 = action_to_states(query_p2.single()?);

    let _ = control_sender.send(if !ui_data.swap_input {
        ControlEvent::Inputs(states_p1, states_p2)
    } else {
        ControlEvent::Inputs(states_p2, states_p1)
    });

    Ok(())
}

fn action_to_states(s: &ActionState<InputAction>) -> InputStates {
    les_nes::InputStates {
        a: s.pressed(&InputAction::A),
        b: s.pressed(&InputAction::B),
        select: s.pressed(&InputAction::Select),
        start: s.pressed(&InputAction::Start),
        up: s.pressed(&InputAction::Up),
        down: s.pressed(&InputAction::Down),
        left: s.pressed(&InputAction::Left),
        right: s.pressed(&InputAction::Right),
    }
}

fn pick_rom(sender: Res<ControlSenderRes>, mut messages: MessageReader<PickRom>) {
    if messages.read().next().is_some() {
        let sender = sender.0.clone();
        bevy::tasks::IoTaskPool::get()
            .spawn(async move {
                if let Some(handle) = rfd::AsyncFileDialog::new().pick_file().await {
                    let _ = sender.send(ControlEvent::LoadCart(handle.read().await));
                }
            })
            .detach();
    }
}
