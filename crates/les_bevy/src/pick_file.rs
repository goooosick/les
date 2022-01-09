use bevy::{prelude::*, tasks::IoTaskPool};
use crossbeam_channel::{Receiver, Sender};
use rfd::AsyncFileDialog;

pub struct PickFilePlugin;

impl Plugin for PickFilePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RequestFile>()
            .add_event::<SelectFile>()
            .add_startup_system(init_chan)
            .add_system(open_dialog)
            .add_system(poll_files);
    }
}

pub struct RequestFile;
pub struct SelectFile(pub Vec<u8>);

fn init_chan(mut commands: Commands) {
    let (sender, receiver) = crossbeam_channel::bounded::<SelectFile>(1);

    commands.insert_resource(sender);
    commands.insert_resource(receiver);
}

fn open_dialog(
    task_pool: Res<IoTaskPool>,
    sender: Res<Sender<SelectFile>>,
    mut events: EventReader<RequestFile>,
) {
    for _ in events.iter() {
        let sender = sender.clone();
        task_pool
            .spawn(async move {
                if let Some(handle) = AsyncFileDialog::new().pick_file().await {
                    let mut data = SelectFile(handle.read().await);
                    while let Err(e) = sender.try_send(data) {
                        data = e.into_inner();
                    }
                }
            })
            .detach();
    }
}

fn poll_files(receiver: Res<Receiver<SelectFile>>, mut events: EventWriter<SelectFile>) {
    while let Ok(data) = receiver.try_recv() {
        events.send(data);
    }
}
