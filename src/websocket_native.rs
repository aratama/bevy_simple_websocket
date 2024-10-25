#![cfg(not(target_arch = "wasm32"))]

use crate::websocket_shared::*;
use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use futures_util::{future, pin_mut, SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Default)]
pub struct WebSocketInstance {
    pub stdin_tx: Option<futures_channel::mpsc::UnboundedSender<Message>>,
    pub open: bool,
    pub sender: Option<crossbeam_channel::Sender<Message>>,
    pub receiver: Option<crossbeam_channel::Receiver<Message>>,
}

// This system reads from the receiver and sends events to Bevy
pub fn read_stream_native(
    mut events: EventWriter<ServerMessage>,
    instance: NonSendMut<WebSocketInstance>,
) {
    if let Some(receiver) = &instance.receiver {
        for item in receiver.try_iter() {
            // println!("Received message: {:?}", item);
            match item {
                Message::Text(s) => {
                    events.send(ServerMessage::String(s.clone()));
                }
                Message::Binary(b) => {
                    events.send(ServerMessage::Binary(b.clone()));
                }
                Message::Close(_) => {
                    events.send(ServerMessage::Close);
                }
                _ => {}
            }
        }
    }
}

pub fn write_message_native(
    mut instance: NonSendMut<WebSocketInstance>,
    mut events: EventReader<ClientMessage>,
    runtime: ResMut<TokioTasksRuntime>,
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

                let url_clone = url.clone();

                // https://github.com/snapview/tokio-tungstenite/blob/master/examples/client.rs
                runtime.spawn_background_task(|mut ctx| async move {
                    let (sender, receiver) = crossbeam_channel::unbounded::<Message>();

                    let sender_clone = sender.clone();

                    let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded::<Message>();

                    println!("Connecting to WebSocket at {}", url_clone);

                    let (ws_stream, _response) =
                        connect_async(url_clone).await.expect("can't connect");

                    let (write, read) = ws_stream.split();

                    let stdin_to_ws = stdin_rx.map(Ok).forward(write);

                    let ws_to_stdout = {
                        read.for_each(|message| async {
                            sender.send(message.unwrap()).unwrap();
                        })
                    };

                    ctx.run_on_main_thread(move |ctx| {
                        let world = ctx.world;
                        world.insert_non_send_resource(WebSocketInstance {
                            stdin_tx: Some(stdin_tx),
                            open: true,
                            sender: Some(sender_clone),
                            receiver: Some(receiver),
                        });
                        world.send_event(ServerMessage::Open);

                        println!("Connected to the server");
                    })
                    .await;

                    pin_mut!(stdin_to_ws, ws_to_stdout);
                    future::select(stdin_to_ws, ws_to_stdout).await;
                });
            }
            ClientMessage::String(s) => {
                if let Some(ref mut stdin_tx) = instance.stdin_tx {
                    println!("Sending message: {}", s);
                    stdin_tx
                        .unbounded_send(Message::Text(s.clone()))
                        .expect("unbounded_send failed at ClientMessage::String");
                    println!("Message sent");
                } else {
                    println!("Sender is None");
                }
            }
            ClientMessage::Binary(b) => {
                if let Some(ref mut stdin_tx) = instance.stdin_tx {
                    stdin_tx
                        .unbounded_send(Message::Binary(b.clone()))
                        .expect("unbounded_send failed at ClientMessage::Binary");
                }
            }
            ClientMessage::Close => {
                if let Some(ref mut stdin_tx) = instance.stdin_tx {
                    stdin_tx
                        .unbounded_send(Message::Close(None))
                        .expect("unbounded_send failed at ClientMessage::Close");
                }
            }
        }
    }
}

#[macro_export]
macro_rules! console_log {
    ($($arg:tt)*) => (println!($($arg)*));
}

#[macro_export]
macro_rules! console_debug {
    ($($arg:tt)*) => (dbg!($($arg)*));
}

#[macro_export]
macro_rules! console_error {
    ($($arg:tt)*) => (dbg!($($arg)*));
}
