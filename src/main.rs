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

#[derive(Resource)]
struct Settings {
    sleep: u32,
}

#[derive(Component)]
struct SleepSetting(u32);

impl Default for Settings {
    fn default() -> Self {
        Self { sleep: 60 }
    }
}

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
        .add_systems(FixedUpdate, (process_message, update, button_system))
        .init_resource::<Settings>()
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

    let sleeps = [2, 3, 4, 5, 10, 20, 60];

    for (i, sleep) in sleeps.iter().enumerate() {
        commands
            .spawn((
                SleepSetting(i as u32),
                ButtonBundle {
                    style: Style {
                        top: Val::Px(100.0 + 50.0 * i as f32),
                        left: Val::Px(50.0),
                        width: Val::Px(50.0),
                        height: Val::Px(30.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    border_color: BorderColor(Color::WHITE),
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section(
                    format!("{}", sleep),
                    TextStyle {
                        font_size: 20.0,
                        color: Color::srgb(0.9, 0.9, 0.9),
                        ..default()
                    },
                ));
            });
    }
}

fn update(
    keys: Res<ButtonInput<KeyCode>>,
    mut writer: EventWriter<WebSocketWriter>,
    mut self_query: Query<(&Player, &mut Transform), With<SelfPlayer>>,
    instance: NonSend<WebSocketInstance>,
    frame_count: Res<FrameCount>,
    settings: Res<Settings>,
) {
    let l = to_sign(&keys, KeyCode::KeyA);
    let d = to_sign(&keys, KeyCode::KeyD);
    for (player, mut transform) in self_query.iter_mut() {
        transform.translation.x += (d - l) * 2.0;

        if instance.opened && frame_count.0 % settings.sleep == 0 {
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
                if let Ok(msg) = serde_json::from_str::<PlayerMessage>(message.as_str()) {
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

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut SleepSetting),
        (Changed<Interaction>, With<Button>),
    >,
    mut settings: ResMut<Settings>,
) {
    for (interaction, sleep_of_button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                settings.sleep = sleep_of_button.0;
            }
            Interaction::Hovered => {}
            Interaction::None => {}
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
