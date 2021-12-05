use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;
use rfd::FileDialog;
use std::path::PathBuf;

pub struct PickFilePlugin;

impl Plugin for PickFilePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<RequestFile>()
            .add_event::<SelectFile>()
            .add_system(open_dialog.system())
            .add_system(poll_tasks.system());
    }
}

pub struct RequestFile;
pub struct SelectFile(pub PathBuf);

struct PickResult(Option<PathBuf>);

fn open_dialog(
    mut commands: Commands,
    task_pool: Res<AsyncComputeTaskPool>,
    mut events: EventReader<RequestFile>,
) {
    for _ in events.iter() {
        commands
            .spawn()
            .insert(task_pool.spawn(async move { PickResult(FileDialog::new().pick_file()) }));
        break;
    }
}

fn poll_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut Task<PickResult>)>,
    mut events: EventWriter<SelectFile>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut *task)) {
            if let PickResult(Some(path)) = result {
                events.send(SelectFile(path));
            }

            commands.entity(entity).despawn();
        }
    }
}
