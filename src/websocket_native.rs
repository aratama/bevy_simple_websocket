#![cfg(not(target_arch = "wasm32"))]

use crate::websocket_shared::*;
use bevy::prelude::*;
use crossbeam_channel::unbounded;
use std::borrow::BorrowMut;
use std::io::Read;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::Message;
use tungstenite::WebSocket;

#[derive(Default)]
pub struct WebSocketInstance {
    pub websocket: Option<Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>>,
    pub open: bool,
}

// This system reads from the receiver and sends events to Bevy
pub fn read_stream_native(
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

pub fn write_message_native(
    mut commands: Commands,
    mut instance: NonSendMut<WebSocketInstance>,
    mut events: EventReader<ClientMessage>,
) {
    for event in events.read() {
        match event {
            ClientMessage::Open(url) => {
                // Close the existing WebSocket if it exists

                // if let Some(ref mut ws) = instance.websocket {
                //     ws.lock().unwrap().close(None).unwrap();
                //     instance.websocket = None;
                //     instance.open = false;
                // }

                println!("Connecting to WebSocket at {}", url);

                let (tx, rx) = unbounded::<ServerMessage>();
                commands.insert_resource(StreamReceiver(rx));

                let url_clone = url.clone();

                let (socket, response) = tungstenite::connect(url_clone).expect("can't connect");

                let arc_socket = Arc::new(Mutex::new(socket));

                instance.websocket = Some(arc_socket.clone());
                instance.open = true;

                thread::spawn(move || {
                    let mut boxed_socket = arc_socket.lock().unwrap();

                    println!("Connected to the server");
                    println!("Response HTTP code: {}", response.status());
                    println!("Response contains the following headers:");
                    for (header, _value) in response.headers() {
                        println!("* {header}");
                    }

                    while boxed_socket.can_read() {
                        println!("Reading from WebSocket");

                        match boxed_socket.read() {
                            Ok(msg) => {
                                println!("Got message from WebSocket {:?}", msg);
                                match msg {
                                    Message::Text(s) => {
                                        tx.send(ServerMessage::String(s)).expect("can't send");
                                    }
                                    Message::Binary(b) => {
                                        tx.send(ServerMessage::Binary(b)).expect("can't send");
                                    }
                                    Message::Close(_) => {
                                        // instance.open = false;
                                        tx.send(ServerMessage::Close).expect("can't send");
                                    }
                                    _ => {}
                                }
                            }
                            Err(e) => {
                                println!("Error reading from WebSocket: {:?}", e);
                                println!("Error reading from WebSocket: {:?}", e);
                            }
                        }
                    }
                });
            }
            ClientMessage::String(s) => {
                // if let Some(ref mut ws) = instance.websocket {
                //     ws.lock().unwrap().send(Message::Text(s.into())).unwrap();
                // }
            }
            ClientMessage::Binary(b) => {
                // if let Some(ref mut ws) = instance.websocket {
                //     ws.lock().unwrap().send(Message::Binary(b.clone())).unwrap();
                // }
            }
            ClientMessage::Close => {
                // if let Some(ref mut ws) = instance.websocket {
                //     ws.lock().unwrap().close(None).unwrap();
                //     instance.websocket = None;
                //     instance.open = false;
                // }
            }
        }
    }
}
