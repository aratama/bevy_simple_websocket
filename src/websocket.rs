#[cfg(target_arch = "wasm32")]
// https://github.com/bevyengine/bevy/blob/main/examples/async_tasks/external_source_external_thread.rs
use bevy::prelude::*;
use crossbeam_channel::{bounded, Receiver};
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::console;
use web_sys::Blob;
use web_sys::Event;
use web_sys::FileReader;
use web_sys::MessageEvent;
use web_sys::WebSocket;

use crate::console_debug;
use crate::console_error;

#[derive(Resource)]
struct WebSocketUrl(String);

// struct WebSocketSettings {
//     websocket: WebSocket
// }

#[derive(Default)]
pub struct WebSocketInstance {
    websocket: Option<WebSocket>,
    pub opened: bool,
}

#[derive(Resource, Deref)]
struct StreamReceiver(Receiver<ServerMessage>);

#[derive(Event)]
pub enum ServerMessage {
    Error(String),
    Open,
    String(String),
    Binary(Vec<u8>),
}

#[derive(Event)]
pub enum ClientMessage {
    String(String),
    Binary(Vec<u8>),
}

fn startup(
    mut commands: Commands,
    url: Res<WebSocketUrl>,
    mut instance: NonSendMut<WebSocketInstance>,
) {
    let (tx, rx) = bounded::<ServerMessage>(10);
    let tx_ = tx.clone();
    let tx__ = tx.clone();
    console_debug!("Connecting to WebSocket at {}", url.0);
    match WebSocket::new(&url.0) {
        Ok(ws) => {
            console_debug!("Connected");

            let on_error = Closure::wrap(Box::new(move |event: Event| {
                web_sys::console::log_1(&JsValue::from(event));
                tx__.send(ServerMessage::Error("ERROR".to_string()))
                    .unwrap();
            }) as Box<dyn FnMut(_)>);
            ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
            on_error.forget();

            let on_open = Closure::<dyn Fn()>::new(move || {
                console::log_1(&"WebSocket opened".into());
                tx_.send(ServerMessage::Open).unwrap();
            });
            ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
            on_open.forget();

            let on_message = Closure::wrap(Box::new(move |event: Event| {
                let message_event = event
                    .dyn_ref::<MessageEvent>()
                    .expect("Event should be a MessageEvent");
                let data = message_event.data();
                if data.is_string() {
                    let message = data.as_string().expect("Data should be a string");
                    console_debug!("WebSocket message: {:?}", message);
                    tx.send(ServerMessage::String(message)).unwrap();
                } else if data.is_instance_of::<Blob>() {
                    let blob = data.dyn_ref::<Blob>().unwrap();
                    let reader = FileReader::new().unwrap();
                    reader.read_as_array_buffer(&blob).unwrap();
                    let tx2 = tx.clone();
                    let onloadend = Closure::wrap(Box::new(move |event: Event| {
                        let target = event.target().unwrap();
                        let reader2: &FileReader = target.dyn_ref().unwrap();
                        let result = reader2.result().unwrap();
                        let array_buffer: js_sys::ArrayBuffer = result.dyn_into().unwrap();
                        let uint8_array = Uint8Array::new(&array_buffer);
                        let vec = uint8_array.to_vec();
                        tx2.send(ServerMessage::Binary(vec)).unwrap();
                    }) as Box<dyn FnMut(_)>);
                    reader.set_onloadend(Some(onloadend.as_ref().unchecked_ref()));
                    onloadend.forget();
                }
            }) as Box<dyn FnMut(_)>);
            ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            on_message.forget();

            *instance = WebSocketInstance {
                websocket: Some(ws),
                opened: false,
            };

            commands.insert_resource(StreamReceiver(rx));
        }
        Err(e) => {
            console_error!("Failed to create WebSocket: {:?}", e);
        }
    }
}

// This system reads from the receiver and sends events to Bevy
fn read_stream(
    receiver: Res<StreamReceiver>,
    mut events: EventWriter<ServerMessage>,
    mut instance: NonSendMut<WebSocketInstance>,
) {
    for from_stream in receiver.try_iter() {
        match from_stream {
            ServerMessage::Open => {
                instance.opened = true;
            }
            _ => {}
        }
        events.send(from_stream);
    }
}

fn write_message(mut events: EventReader<ClientMessage>, ws_nonsend: NonSend<WebSocketInstance>) {
    if let Some(ws) = &ws_nonsend.websocket {
        for event in events.read() {
            match event {
                ClientMessage::String(s) => {
                    ws.send_with_str(s).unwrap();
                }
                ClientMessage::Binary(b) => {
                    ws.send_with_u8_array(b).unwrap();
                }
            }
        }
    }
}

pub struct WebSocketPlugin {
    pub url: String,
}

impl Plugin for WebSocketPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup)
            .add_systems(Update, (read_stream, write_message))
            .add_event::<ServerMessage>()
            .add_event::<ClientMessage>()
            .insert_resource(WebSocketUrl(self.url.clone()))
            .init_non_send_resource::<WebSocketInstance>();
    }
}

async fn blob_into_bytes(blob: &Blob) -> Vec<u8> {
    let array_buffer_promise: JsFuture = blob.array_buffer().into();

    let array_buffer: JsValue = array_buffer_promise
        .await
        .expect("Could not get ArrayBuffer from file");

    js_sys::Uint8Array::new(&array_buffer).to_vec()
}
