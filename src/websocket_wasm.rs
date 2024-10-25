#![cfg(target_arch = "wasm32")]

use crate::websocket_shared::*;
use bevy::prelude::*;
use crossbeam_channel::unbounded;
use js_sys::ArrayBuffer;
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use web_sys::BinaryType;
use web_sys::Event;
use web_sys::MessageEvent;
use web_sys::WebSocket;

#[derive(Default)]
pub(crate) struct WebSocketInstance {
    pub websocket: Option<WebSocket>,
}

// This system reads from the receiver and sends events to Bevy
pub(crate) fn read_stream_wasm(
    receiver_option: Option<Res<StreamReceiver>>,
    mut events: EventWriter<ServerMessage>,
    mut instance: NonSendMut<WebSocketInstance>,
    mut state: ResMut<WebSocketState>,
) {
    if let Some(receiver) = receiver_option {
        for from_stream in receiver.try_iter() {
            match from_stream {
                ServerMessage::Open => {
                    state.ready_state = ReadyState::OPEN;
                }
                ServerMessage::Close => {
                    instance.websocket = None;
                    state.ready_state = ReadyState::CLOSED;
                }
                _ => {}
            }
            events.send(from_stream);
        }
    }
}

pub(crate) fn write_message_wasm(
    mut commands: Commands,
    mut instance: NonSendMut<WebSocketInstance>,
    mut events: EventReader<ClientMessage>,
    mut state: ResMut<WebSocketState>,
) {
    for event in events.read() {
        match event {
            ClientMessage::Open(url) => {
                // Close the existing WebSocket if it exists
                if let Some(ws) = &instance.websocket {
                    ws.close().expect("Failed to close WebSocket");
                    instance.websocket = None;
                    state.ready_state = ReadyState::CLOSING;
                }

                let (tx, rx) = unbounded::<ServerMessage>();
                let tx_err = tx.clone();
                let tx_open = tx.clone();
                let tx_close = tx.clone();
                info!("Connecting to WebSocket at {}", url);
                match WebSocket::new(&url) {
                    Ok(ws) => {
                        debug!("Connected");

                        ws.set_binary_type(BinaryType::Arraybuffer);

                        let on_error = Closure::wrap(Box::new(move |event: Event| {
                            error!("WebSocket error: {:?}", event);
                            tx_err
                                .send(ServerMessage::Error("ERROR".to_string()))
                                .unwrap();
                        })
                            as Box<dyn FnMut(_)>);
                        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
                        on_error.forget();

                        let on_open = Closure::<dyn Fn()>::new(move || {
                            debug!("WebSocket opened");
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
                                tx.send(ServerMessage::String(message)).unwrap();
                            } else if data.is_instance_of::<ArrayBuffer>() {
                                let array_buffer = data.dyn_ref::<ArrayBuffer>().unwrap();
                                let uint8_array = Uint8Array::new(&array_buffer);
                                let vec = uint8_array.to_vec();
                                tx.send(ServerMessage::Binary(vec)).unwrap();
                            } else {
                                error!("Unexpected WebSocket message type");
                            }
                        })
                            as Box<dyn FnMut(_)>);
                        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
                        on_message.forget();

                        let on_close = Closure::<dyn Fn()>::new(move || {
                            debug!("WebSocket closed");
                            tx_close.send(ServerMessage::Close).unwrap();
                        });
                        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
                        on_close.forget();

                        *instance = WebSocketInstance {
                            websocket: Some(ws),
                        };

                        commands.insert_resource(StreamReceiver(rx));
                    }
                    Err(e) => {
                        error!("Failed to create WebSocket: {:?}", e);
                    }
                }
            }
            ClientMessage::String(s) => {
                if let Some(ws) = &instance.websocket {
                    if ws.ready_state() == WebSocket::OPEN {
                        ws.send_with_str(s)
                            .expect("Failed to send WebSocket string message");
                    } else {
                        warn!("WebSocket is not open");
                    }
                }
            }
            ClientMessage::Binary(b) => {
                if let Some(ws) = &instance.websocket {
                    if ws.ready_state() == WebSocket::OPEN {
                        ws.send_with_u8_array(b)
                            .expect("Failed to send WebSocket binary message");
                    } else {
                        warn!("WebSocket is not open");
                    }
                }
            }
            ClientMessage::Close => {
                if let Some(ws) = &instance.websocket {
                    ws.close().expect("Failed to close WebSocket");
                    instance.websocket = None;
                    state.ready_state = ReadyState::CLOSED;
                }
            }
        }
    }
}
