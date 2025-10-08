use bevy::prelude::*;
use les_nes::{Bus, Cartridge, Cpu, InputStates};
use std::sync::{Arc, Mutex};

mod audio;
mod ui;

type ControlReceiver = crossbeam_channel::Receiver<ControlEvent>;
type ControlSender = crossbeam_channel::Sender<ControlEvent>;

enum ControlEvent {
    LoadCart(Vec<u8>),
    AudioCtrl([bool; 5]),
    Inputs(InputStates, InputStates),
    Reset,
    Pause,
    Step,
}

struct EmuContext {
    pub cpu: Cpu,
    pub bus: Bus,
    pub pause: bool,
    pub step: bool,
    pub cnotrol_events: ControlReceiver,
}

type SharedEmuContext = Arc<Mutex<EmuContext>>;

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let (sender, receiver) = crossbeam_channel::unbounded();
    let emu = {
        let mut bus = Bus::new(Cartridge::empty());
        let mut cpu = Cpu::default();
        bus.reset(&mut cpu);

        Arc::new(Mutex::new(EmuContext {
            cpu,
            bus,
            pause: false,
            step: false,
            cnotrol_events: receiver,
        }))
    };

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "LES!".to_owned(),
            present_mode: bevy::window::PresentMode::AutoVsync,
            #[cfg(target_arch = "wasm32")]
            canvas: Some("#viewport".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    }))
    .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
    .add_plugins(ui::UiPlugin {
        emu: emu.clone(),
        control_sender: sender,
    })
    .add_plugins(audio::AudioRunnerPlugin { emu })
    .run();
}
