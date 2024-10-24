#[cfg(target_arch = "wasm32")]
// https://github.com/bevyengine/bevy/blob/main/examples/async_tasks/external_source_external_thread.rs
use bevy::prelude::*;
use crossbeam_channel::unbounded;
use crossbeam_channel::Receiver;
use js_sys::ArrayBuffer;
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use web_sys::console;
use web_sys::BinaryType;
use web_sys::Event;
use web_sys::MessageEvent;
use web_sys::WebSocket;

use crate::console_debug;
use crate::console_error;

#[derive(Default)]
pub struct WebSocketInstance {
    websocket: Option<WebSocket>,
    pub open: bool,
}

#[derive(Resource, Deref)]
struct StreamReceiver(Receiver<ServerMessage>);

#[derive(Event)]
pub enum ServerMessage {
    Error(String),
    Open,
    String(String),
    Binary(Vec<u8>),
    Close,
}

#[derive(Event)]
pub enum ClientMessage {
    Open(String),
    String(String),
    Binary(Vec<u8>),
    Close,
}

// This system reads from the receiver and sends events to Bevy
fn read_stream(
    receiver_option: Option<Res<StreamReceiver>>,
    mut events: EventWriter<ServerMessage>,
    mut instance: NonSendMut<WebSocketInstance>,
) {
    if let Some(receiver) = receiver_option {
        for from_stream in receiver.try_iter() {
            match from_stream {
                ServerMessage::Open => {
                    instance.open = true;
                }
                _ => {}
            }
            events.send(from_stream);
        }
    }
}

fn write_message(
    mut commands: Commands,
    mut instance: NonSendMut<WebSocketInstance>,
    mut events: EventReader<ClientMessage>,
) {
    for event in events.read() {
        match event {
            ClientMessage::Open(url) => {
                // Close the existing WebSocket if it exists
                if let Some(ws) = &instance.websocket {
                    ws.close().unwrap();
                    instance.websocket = None;
                    instance.open = false;
                }

                let (tx, rx) = unbounded::<ServerMessage>();
                let tx_err = tx.clone();
                let tx_open = tx.clone();
                let tx_close = tx.clone();
                console_debug!("Connecting to WebSocket at {}", url);
                match WebSocket::new(&url) {
                    Ok(ws) => {
                        console_debug!("Connected");

                        ws.set_binary_type(BinaryType::Arraybuffer);

                        let on_error = Closure::wrap(Box::new(move |event: Event| {
                            web_sys::console::log_1(&JsValue::from(event));
                            tx_err
                                .send(ServerMessage::Error("ERROR".to_string()))
                                .unwrap();
                        })
                            as Box<dyn FnMut(_)>);
                        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
                        on_error.forget();

                        let on_open = Closure::<dyn Fn()>::new(move || {
                            console::log_1(&"WebSocket opened".into());
                            tx_open.send(ServerMessage::Open).unwrap();
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
                            } else if data.is_instance_of::<ArrayBuffer>() {
                                let array_buffer = data.dyn_ref::<ArrayBuffer>().unwrap();
                                let uint8_array = Uint8Array::new(&array_buffer);
                                let vec = uint8_array.to_vec();
                                tx.send(ServerMessage::Binary(vec)).unwrap();
                            } else {
                                console_error!("Unexpected WebSocket message type");
                            }
                        })
                            as Box<dyn FnMut(_)>);
                        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
                        on_message.forget();

                        let on_close = Closure::<dyn Fn()>::new(move || {
                            console::log_1(&"WebSocket closed".into());
                            tx_close.send(ServerMessage::Close).unwrap();
                        });
                        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
                        on_close.forget();

                        *instance = WebSocketInstance {
                            websocket: Some(ws),
                            open: false,
                        };

                        commands.insert_resource(StreamReceiver(rx));
                    }
                    Err(e) => {
                        console_error!("Failed to create WebSocket: {:?}", e);
                    }
                }
            }
            ClientMessage::String(s) => {
                if let Some(ws) = &instance.websocket {
                    ws.send_with_str(s).unwrap();
                }
            }
            ClientMessage::Binary(b) => {
                if let Some(ws) = &instance.websocket {
                    ws.send_with_u8_array(b).unwrap();
                }
            }
            ClientMessage::Close => {
                if let Some(ws) = &instance.websocket {
                    ws.close().unwrap();
                    instance.websocket = None;
                    instance.open = false;
                }
            }
        }
    }
}

pub struct WebSocketPlugin;

impl Plugin for WebSocketPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (read_stream, write_message))
            .add_event::<ServerMessage>()
            .add_event::<ClientMessage>()
            .init_non_send_resource::<WebSocketInstance>();
    }
}
