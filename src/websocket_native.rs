#![cfg(not(target_arch = "wasm32"))]

use crate::websocket_shared::*;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use bevy::tasks::Task;
use bevy_tokio_tasks::TokioTasksRuntime;
use crossbeam_channel::unbounded;
use futures_util::{future, pin_mut, SinkExt, StreamExt};
use tokio::io::AsyncWriteExt;
use tokio_tungstenite::{connect_async, tungstenite::Message};
#[derive(Default)]
pub struct WebSocketInstance {
    pub stdin_tx: Option<futures_channel::mpsc::UnboundedSender<Message>>,
    // pub receiver: Option<futures_channel::mpsc::UnboundedReceiver<Message>>,
    pub open: bool,

    pub sender: Option<crossbeam_channel::Sender<Message>>,
    pub receiver: Option<crossbeam_channel::Receiver<Message>>,
}

// This system reads from the receiver and sends events to Bevy
pub fn read_stream_native(
    receiver_option: Option<Res<StreamReceiver>>,
    mut events: EventWriter<ServerMessage>,
    mut instance: NonSendMut<WebSocketInstance>,
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

#[derive(Component)]
pub struct ComputeTransform(Task<CommandQueue>);

pub fn write_message_native(
    mut commands: Commands,
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
                runtime.spawn_background_task(|mut _ctx| async move {
                    let (sender, receiver) = crossbeam_channel::unbounded::<Message>();

                    let sender_clone = sender.clone();

                    let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded::<Message>();

                    println!("Connecting to WebSocket at {}", url_clone);

                    let (ws_stream, response) =
                        connect_async(url_clone).await.expect("can't connect");
                    println!("Connected to the server");
                    println!("Response HTTP code: {}", response.status());
                    println!("Response contains the following headers:");
                    for (header, _value) in response.headers() {
                        println!("* {header}");
                    }

                    let (mut write, read) = ws_stream.split();

                    write
                        .send(Message::Text("hogehoge".to_string()))
                        .await
                        .unwrap();

                    let stdin_to_ws = stdin_rx.map(Ok).forward(write);

                    stdin_tx
                        .unbounded_send(Message::Text("piyopiyo".to_string()))
                        .expect("unbounded_send failed");

                    let ws_to_stdout = {
                        read.for_each(|message| async {
                            sender.send(message.unwrap()).unwrap();
                        })
                    };

                    _ctx.run_on_main_thread(move |ctx| {
                        let world = ctx.world;
                        world.insert_non_send_resource(WebSocketInstance {
                            stdin_tx: Some(stdin_tx),
                            open: true,
                            sender: Some(sender_clone),
                            receiver: Some(receiver),
                        });
                    })
                    .await;

                    println!("pub mut");
                    pin_mut!(stdin_to_ws, ws_to_stdout);
                    println!("select");
                    future::select(stdin_to_ws, ws_to_stdout).await;
                    println!("ok");
                });
            }
            ClientMessage::String(s) => {
                if let Some(ref mut stdin_tx) = instance.stdin_tx {
                    println!("Sending message: {}", s);
                    stdin_tx.unbounded_send(Message::Text(s.clone()));
                    println!("Message sent");
                } else {
                    println!("Sender is None");
                }
            }
            ClientMessage::Binary(b) => {
                if let Some(ref mut stdin_tx) = instance.stdin_tx {
                    stdin_tx.unbounded_send(Message::Binary(b.clone()));
                }
            }
            ClientMessage::Close => {
                if let Some(ref mut stdin_tx) = instance.stdin_tx {
                    stdin_tx.unbounded_send(Message::Close(None));
                }
            }
        }
    }
}
