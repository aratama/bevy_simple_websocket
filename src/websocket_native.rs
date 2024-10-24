#![cfg(not(target_arch = "wasm32"))]

use crate::websocket_shared::*;
use bevy::prelude::*;
use std::net::TcpStream;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::Message;
use tungstenite::WebSocket;

#[derive(Default)]
pub struct WebSocketInstance {
    pub websocket: Option<WebSocket<MaybeTlsStream<TcpStream>>>,
    pub open: bool,
}

// This system reads from the receiver and sends events to Bevy
pub fn read_stream_native(
    mut events: EventWriter<ServerMessage>,
    mut instance: NonSendMut<WebSocketInstance>,
) {
    if let Some(ref mut ws) = instance.websocket {
        println!("Checking if WebSocket can read");
        while ws.can_read() {
            println!("Reading from WebSocket");
            match ws.read() {
                Ok(msg) => {
                    println!("Got message from WebSocket {:?}", msg);
                    match msg {
                        Message::Text(s) => {
                            events.send(ServerMessage::String(s));
                        }
                        Message::Binary(b) => {
                            events.send(ServerMessage::Binary(b));
                        }
                        Message::Close(_) => {
                            // instance.open = false;
                            events.send(ServerMessage::Close);
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    println!("Error reading from WebSocket: {:?}", e);
                }
            }
        }
    }
}

pub fn write_message_native(
    mut instance: NonSendMut<WebSocketInstance>,
    mut events: EventReader<ClientMessage>,
) {
    for event in events.read() {
        match event {
            ClientMessage::Open(url) => {
                println!("Connecting to WebSocket at {}", url);
                let (socket, response) = tungstenite::connect(url).expect("can't connect");
                println!("Connected to the server");
                println!("Response HTTP code: {}", response.status());
                println!("Response contains the following headers:");
                for (header, _value) in response.headers() {
                    println!("* {header}");
                }

                instance.websocket = Some(socket);
                instance.open = true;
            }
            ClientMessage::String(s) => {
                if let Some(ref mut ws) = instance.websocket {
                    ws.send(Message::Text(s.into())).unwrap();
                }
            }
            ClientMessage::Binary(b) => {
                if let Some(ref mut ws) = instance.websocket {
                    ws.send(Message::Binary(b.clone())).unwrap();
                }
            }
            ClientMessage::Close => {
                if let Some(ref mut ws) = instance.websocket {
                    ws.close(None).unwrap();
                    instance.websocket = None;
                    instance.open = false;
                }
            }
        }
    }
}
