mod console;
mod websocket;
mod websocket_shared;

#[cfg(target_arch = "wasm32")]
mod websocket_wasm;

#[cfg(not(target_arch = "wasm32"))]
mod websocket_native;

use bevy::asset::AssetMetaCheck;
use bevy::core::FrameCount;
use bevy::prelude::*;
use dotenvy_macro::dotenv;
use rand;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;
use websocket::WebSocketPlugin;
use websocket_shared::*;

#[cfg(target_arch = "wasm32")]
use websocket_wasm::WebSocketInstance;

#[cfg(not(target_arch = "wasm32"))]
use websocket_native::WebSocketInstance;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct PlayerMessage {
    uuid: Uuid,
    position: Vec2,
}

#[derive(Component)]
struct OtherPlayer {
    uuid: Uuid,
    last_update: FrameCount,
}

#[derive(Component)]
struct SelfPlayer {
    uuid: Uuid,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            // https://github.com/bevyengine/bevy/issues/10157
            meta_check: AssetMetaCheck::Never,
            ..default()
        }))
        .add_plugins(WebSocketPlugin)
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, (process_message, update))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_setver: Res<AssetServer>,
    mut writer: EventWriter<ClientMessage>,
) {
    let uuid = Uuid::new_v4();
    // console_log!("self uuid: {:?}", uuid);

    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        SelfPlayer { uuid },
        SpriteBundle {
            texture: asset_setver.load("icon.png"),
            transform: Transform::from_xyz(
                500.0 * (rand::random::<f32>() - 0.5),
                500.0 * (rand::random::<f32>() - 0.5),
                0.,
            ),
            ..default()
        },
    ));

    let url = dotenv!("url");
    writer.send(ClientMessage::Open(url.to_string()));
}

fn update(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut writer: EventWriter<ClientMessage>,
    mut self_query: Query<(&SelfPlayer, &mut Transform)>,
    others_query: Query<(Entity, &mut OtherPlayer)>,
    instance: NonSend<WebSocketInstance>,
    frame_count: Res<FrameCount>,
) {
    let w = to_sign(&keys, KeyCode::KeyW);
    let a = to_sign(&keys, KeyCode::KeyA);
    let s = to_sign(&keys, KeyCode::KeyS);
    let d = to_sign(&keys, KeyCode::KeyD);
    for (player, mut transform) in self_query.iter_mut() {
        transform.translation.x += (d - a) * 2.0;
        transform.translation.y += (w - s) * 2.0;

        let message_interval = 1;

        if instance.open && frame_count.0 % message_interval == 0 {
            let value = PlayerMessage {
                uuid: player.uuid,
                position: Vec2::new(transform.translation.x, transform.translation.y),
            };
            let bin = bincode::serialize(&value).unwrap();
            // console_log!("send bincode");
            writer.send(ClientMessage::Binary(bin));
        }
    }

    for (entity, player) in others_query.iter() {
        if 120 < (frame_count.0 - player.last_update.0) {
            // console_log!("despawn other player {:?}", player.uuid);
            commands.entity(entity).despawn();
        }
    }
}

fn process_message(
    mut commands: Commands,
    asset_setver: Res<AssetServer>,
    mut query: Query<(&mut OtherPlayer, &mut Transform)>,
    frame_count: Res<FrameCount>,
    mut reader: EventReader<ServerMessage>,
    mut writer: EventWriter<ClientMessage>,
) {
    for event in reader.read() {
        match event {
            ServerMessage::Error(err) => {
                // console_log!("WebSocket error: {:?}", err)
            }
            ServerMessage::Open => {
                // console_log!("WebSocket opened");
                writer.send(ClientMessage::String("hello, server".to_string()));
            }
            ServerMessage::String(message) => {
                // console_log!("WebSocket string message: {:?}", message);
            }
            ServerMessage::Binary(bytes) => {
                let msg = bincode::deserialize::<PlayerMessage>(bytes).unwrap();
                // console_log!("WebSocket binary message({:?}): {:?}", bytes.len(), msg);

                // sync position of existing player
                let mut synced = false;
                for (mut p, mut t) in query.iter_mut() {
                    if p.uuid == msg.uuid {
                        t.translation.x = msg.position.x;
                        t.translation.y = msg.position.y;
                        p.last_update = frame_count.clone();
                        synced = true;
                        break;
                    }
                }

                // spawn new player
                if !synced {
                    // console_log!("spawning other player {:?}", msg.uuid);
                    commands.spawn((
                        OtherPlayer {
                            uuid: msg.uuid,
                            last_update: frame_count.clone(),
                        },
                        SpriteBundle {
                            texture: asset_setver.load("icon.png"),
                            transform: Transform::from_xyz(msg.position.x, 0., 0.),
                            ..default()
                        },
                    ));
                }
            }
            ServerMessage::Close => {
                // console_log!("WebSocket closed");
            }
        }
    }
}

fn to_sign(keys: &Res<ButtonInput<KeyCode>>, code: KeyCode) -> f32 {
    if keys.pressed(code) {
        1.0
    } else {
        0.0
    }
}
