#![allow(clippy::type_complexity)]

use std::process::Stdio;

use bevy::{
    prelude::*,
    utils::{smallvec::SmallVec, Uuid},
};

use bevy_simple_text_input::{TextInputBundle, TextInputPlugin, TextInputSubmitEvent};
use bevy_world_sync::iceoryx2::prelude::*;
use cew::Lay;

fn main() -> cew::U {
    cew::init()?;
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(
            bevy_panic_handler::PanicHandler::new()
                .take_call_from_existing()
                .build(),
        )
        .add_plugins(WorldSyncConsumerPlugin)
        .add_plugins(TextInputPlugin)
        .insert_resource(Cwd(None))
        .add_systems(Startup, startup_ui)
        .add_systems(Update, (thingbutton, listener, bloomin_doomin))
        .run();

    Ok(())
}

#[derive(Component)]
struct GameSubProcess(std::process::Child);

#[derive(Resource)]
struct Cwd(Option<std::path::PathBuf>);

#[derive(Component)]
struct ThingButton;
fn thingbutton(
    mut commands: Commands,
    mut thing_buttons: Query<
        (Entity, &mut BackgroundColor, &Interaction),
        (Changed<Interaction>, With<ThingButton>, With<Node>),
    >,
    sync_key: Res<WorldSyncKey>,
    cwd: Res<Cwd>,
) {
    for (entity, mut bg, interaction) in thing_buttons.iter_mut() {
        match interaction {
            Interaction::Pressed => {
                if cwd.0.is_some() {
                    commands.entity(entity).despawn_recursive();
                    commands.spawn(GameSubProcess(
                        std::process::Command::new("cargo")
                            .arg("run")
                            .env("BEVY_WORLD_SYNC_SERVICE", /*&sync_key.0*/ "fkey_2")
                            .env("RUST_BACKTRACE", "1")
                            .current_dir(cwd.0.as_ref().unwrap())
                            // .stdout(Stdio::piped())
                            // .stderr(Stdio::piped())
                            // .stdin(Stdio::piped())
                            .spawn()
                            .unwrap(),
                    ));
                }
            }
            Interaction::Hovered => *bg = BackgroundColor(Color::DARK_GREEN),
            Interaction::None => {
                *bg = BackgroundColor(Color::SEA_GREEN);
            }
        }
    }
}

fn startup_ui(mut commands: Commands, key: Res<WorldSyncKey>) {
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                height: Val::Percent(100.0),
                width: Val::Percent(100.0),
                ..Default::default()
            },
            background_color: BackgroundColor(Color::MIDNIGHT_BLUE.with_a(0.6)),
            ..Default::default()
        })
        .with_children(|c| {
            c.spawn(TextBundle {
                text: Text::from_section(
                    key.0.clone(),
                    TextStyle {
                        color: Color::RED,
                        font_size: 28.0,
                        ..Default::default()
                    },
                ),
                ..Default::default()
            });
            c.spawn(ButtonBundle::default())
                .with_children(|c| {
                    c.spawn(TextBundle::from_section(
                        "Enable Thing",
                        TextStyle::default(),
                    ));
                })
                .insert(ThingButton);
            c.spawn((
                NodeBundle::default().lay(|f| f.style.width = Val::Px(200.0)),
                TextInputBundle::default(),
            ));
            c.spawn(TextBundle {
                text: Text::from_section(
                    "",
                    TextStyle {
                        color: Color::RED,
                        font_size: 28.0,
                        ..Default::default()
                    },
                ),
                ..Default::default()
            })
            .insert(DebugText);
        });
}

#[derive(Component)]
struct DebugText;
fn bloomin_doomin(
    mut commands: Commands,
    mut subproc: Query<(Entity, &mut GameSubProcess)>,
    mut query: Query<&mut Text, With<DebugText>>,
    sync_channel: NonSendMut<ActiveWorldSyncChannel>,
) {
    let Ok((entity, mut proc)) = subproc.get_single_mut() else {
        return;
    };
    let proc = &mut proc.0;

    if let Some(status) = proc.try_wait().unwrap() {
        commands.entity(entity).despawn();
        if !status.success() {
            error!("Subprocess exited unexpectedly: {status}.");
        } else {
            info!("Subprocess exited: {status}.");
        }
        return;
    }

    let mut msgs = SmallVec::<[Vec<u8>; 1]>::new();
    let mut currmsg = vec![];
    while let Some(msg) = sync_channel.0.receive().unwrap() {
        let (c, end) = msg.get_chunk();
        info!("Recieved: {}", String::from_utf8_lossy(c));
        currmsg.extend(c);
        if end {
            msgs.push(currmsg);
            currmsg = vec![];
        }
    }
    if msgs.is_empty() {
        warn!("No messages?");
        return;
    }

    for state in &msgs {
        info!("Recieved msg: {}", String::from_utf8_lossy(state));
    }

    for mut text in query.iter_mut() {
        *text = Text::from_section(String::from_utf8_lossy(msgs.last().unwrap()), default());
    }
}

fn listener(mut events: EventReader<TextInputSubmitEvent>, mut path: ResMut<Cwd>) {
    for event in events.read() {
        info!("Setting Cwd to {}", event.value);
        *path = Cwd(Some(std::path::PathBuf::from(&event.value)));
    }
}

struct WorldSyncConsumerPlugin;
#[derive(Resource)]
struct WorldSyncKey(String);
struct ActiveWorldSyncChannel(
    bevy_world_sync::iceoryx2::port::subscriber::Subscriber<
        zero_copy::Service,
        bevy_world_sync::Bytes,
    >,
);

impl Plugin for WorldSyncConsumerPlugin {
    fn build(&self, app: &mut App) {
        let uuid_str = Uuid::new_v4().as_simple().to_string();
        let srevice_name = ServiceName::new(/*&uuid_str*/ "fkey_2").unwrap();
        let builder = zero_copy::Service::new(&srevice_name)
            .publish_subscribe()
            .open_or_create()
            .unwrap();
        let subsscriber = builder.subscriber().create().unwrap();
        app.insert_resource(WorldSyncKey(uuid_str));
        app.insert_non_send_resource(ActiveWorldSyncChannel(subsscriber));
    }
}
