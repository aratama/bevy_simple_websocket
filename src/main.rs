mod console;
mod websocket;

use bevy::asset::AssetMetaCheck;
use bevy::core::FrameCount;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use websocket::WebSocketInstance;
use websocket::WebSocketPlugin;
use websocket::WebSocketReader;
use websocket::WebSocketWriter;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct PlayerMessage {
    uuid: Uuid,
    position: Vec2,
}

#[derive(Component)]
struct Player {
    uuid: Uuid,
}

#[derive(Component)]
struct SelfPlayer;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            // https://github.com/bevyengine/bevy/issues/10157
            meta_check: AssetMetaCheck::Never,
            ..default()
        }))
        .add_plugins(WebSocketPlugin {
            // url: "ws://localhost:8080".to_string(),
            url: "https://magia-server-38847751193.asia-northeast1.run.app".to_string(),
        })
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, (process_message, update))
        .run();
}

fn setup(mut commands: Commands, asset_setver: Res<AssetServer>) {
    let uuid = Uuid::new_v4();
    console_log!("self uuid: {:?}", uuid);

    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        SelfPlayer,
        Player { uuid },
        SpriteBundle {
            texture: asset_setver.load("icon.png"),
            transform: Transform::from_xyz(100., 0., 0.),
            ..default()
        },
    ));
}

fn update(
    keys: Res<ButtonInput<KeyCode>>,
    mut writer: EventWriter<WebSocketWriter>,
    mut self_query: Query<(&Player, &mut Transform), With<SelfPlayer>>,
    instance: NonSend<WebSocketInstance>,
    frame_count: Res<FrameCount>,
) {
    let l = to_sign(&keys, KeyCode::KeyA);
    let d = to_sign(&keys, KeyCode::KeyD);
    for (player, mut transform) in self_query.iter_mut() {
        transform.translation.x += (d - l) * 2.0;

        if instance.opened && frame_count.0 % 60 == 0 {
            let json = serde_json::to_string(&PlayerMessage {
                uuid: player.uuid,
                position: Vec2::new(transform.translation.x, transform.translation.y),
            })
            .unwrap();
            console_log!("send: {:?}", json.clone());
            writer.send(WebSocketWriter(json));
        }
    }
}

fn process_message(
    mut commands: Commands,
    asset_setver: Res<AssetServer>,
    mut events: EventReader<WebSocketReader>,
    mut query: Query<(&Player, &mut Transform)>,
) {
    for event in events.read() {
        match event {
            WebSocketReader::Error(_) => console_log!("WebSocket error"),
            WebSocketReader::Open => console_log!("WebSocket opened"),
            WebSocketReader::Message(message) => {
                if let Ok(msg) = serde_json::from_str::<PlayerMessage>(message) {
                    // sync position
                    let mut synced = false;
                    for (p, mut t) in query.iter_mut() {
                        if p.uuid == msg.uuid {
                            t.translation.x = msg.position.x;
                            synced = true;
                        }
                    }
                    // spawn other player
                    if !synced {
                        console_log!("spawn other player {:?}", msg.uuid);
                        commands.spawn((
                            Player { uuid: msg.uuid },
                            SpriteBundle {
                                texture: asset_setver.load("icon.png"),
                                transform: Transform::from_xyz(msg.position.x, 0., 0.),
                                ..default()
                            },
                        ));
                    }
                } else {
                    console_log!("WebSocket message: {:?}", message);
                }
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
