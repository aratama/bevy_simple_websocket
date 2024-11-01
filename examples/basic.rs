use bevy::core::FrameCount;
use bevy::prelude::*;
use bevy_simple_websocket::*;
use dotenvy_macro::dotenv;
use rand;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

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
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (500., 300.).into(),
                ..default()
            }),
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
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        SelfPlayer {
            uuid: Uuid::new_v4(),
        },
        SpriteBundle {
            texture: asset_setver.load("icon.png"),
            transform: Transform::from_xyz(
                200.0 * (rand::random::<f32>() - 0.5),
                200.0 * (rand::random::<f32>() - 0.5),
                0.,
            )
            .with_scale(Vec3::splat(0.2)),
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
    state: Res<WebSocketState>,
    frame_count: Res<FrameCount>,
) {
    // update position by key input
    let w = to_sign(&keys, KeyCode::KeyW);
    let a = to_sign(&keys, KeyCode::KeyA);
    let s = to_sign(&keys, KeyCode::KeyS);
    let d = to_sign(&keys, KeyCode::KeyD);
    let speed = 2.0;
    let (player, mut transform) = self_query.single_mut();
    transform.translation.x += (d - a) * speed;
    transform.translation.y += (w - s) * speed;

    // send position to server
    if state.ready_state == ReadyState::OPEN {
        let message = PlayerMessage {
            uuid: player.uuid,
            position: Vec2::new(transform.translation.x, transform.translation.y),
        };
        let bin = bincode::serialize(&message).unwrap();
        writer.send(ClientMessage::Binary(bin));
    }

    // despawn other player if not updated for 30 frames
    for (entity, player) in others_query.iter() {
        if 30 < (frame_count.0 - player.last_update.0) {
            info!("despawn other player {:?}", player.uuid);
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
                error!("WebSocket error: {:?}", err);
            }
            ServerMessage::Open => {
                info!("WebSocket opened");
                writer.send(ClientMessage::String("hello, server".to_string()));
            }
            ServerMessage::String(message) => {
                info!("WebSocket string message: {:?}", message);
            }
            ServerMessage::Binary(bytes) => {
                let msg = bincode::deserialize::<PlayerMessage>(bytes).unwrap();

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
                    info!("spawning other player {:?}", msg.uuid);
                    commands.spawn((
                        OtherPlayer {
                            uuid: msg.uuid,
                            last_update: frame_count.clone(),
                        },
                        SpriteBundle {
                            texture: asset_setver.load("icon.png"),
                            transform: Transform::from_xyz(msg.position.x, 0., 0.)
                                .with_scale(Vec3::splat(0.2)),
                            ..default()
                        },
                    ));
                }
            }
            ServerMessage::Close => {
                info!("WebSocket closed");
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
