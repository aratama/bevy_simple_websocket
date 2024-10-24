#![cfg(not(target_arch = "wasm32"))]

use crate::websocket_shared::*;
use async_std::task::spawn;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use bevy::tasks::futures_lite::future;
use bevy::tasks::{block_on, TaskPool};
use bevy::tasks::{AsyncComputeTaskPool, Task};
use crossbeam_channel::unbounded;
use std::borrow::BorrowMut;
use std::io::Read;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::time::{sleep, Sleep};
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

#[derive(Component)]
pub struct ComputeTransform(Task<CommandQueue>);

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

                let url_clone = url.clone();
                let thread_pool = AsyncComputeTaskPool::get();
                let entity = commands.spawn_empty().id();
                let task = thread_pool.spawn(async move {
                    let mut command_queue = CommandQueue::default();

                    println!("Connecting to WebSocket at {}", url_clone);

                    // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    // tokio の sleep は以下の panic になる
                    // thread 'Async Compute Task Pool (2)' panicked at src\websocket_native.rs:72:21:
                    // there is no reactor running, must be called from the context of a Tokio 1.x runtime

                    async_std::task::sleep(std::time::Duration::from_secs(2)).await;

                    let (socket, response) =
                        tungstenite::connect(url_clone).expect("can't connect");
                    println!("Connected to the server");
                    println!("Response HTTP code: {}", response.status());
                    println!("Response contains the following headers:");
                    for (header, _value) in response.headers() {
                        println!("* {header}");
                    }

                    command_queue.push(move |world: &mut World| {
                        // これがないと Task polled after completion で panic になる
                        world.despawn(entity);

                        println!("WebSockeet initialized");
                        world.insert_non_send_resource(WebSocketInstance {
                            websocket: Some(socket),
                            open: true,
                        });
                    });

                    command_queue
                });
                commands.entity(entity).insert(ComputeTransform(task));
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

pub fn start_reading(mut commands: Commands, mut instance: NonSendMut<WebSocketInstance>) {
    if instance.is_changed() {
        println!("instance added");
        let (tx, rx) = unbounded::<ServerMessage>();
        commands.insert_resource(StreamReceiver(rx));
        let pool = TaskPool::new();
        pool.scope(|s| {
            s.spawn(async {
                println!("Starting WebSocket reading task");
                match instance.websocket {
                    Some(ref mut ws) => {
                        println!("WebSocket is open");
                        while ws.can_read() {
                            println!("Reading from WebSocket");
                            match ws.read() {
                                Ok(msg) => {
                                    println!("Got message from WebSocket {:?}", msg);
                                    match msg {
                                        Message::Text(s) => {
                                            println!("Got text message from WebSocket {:?}", s);
                                            tx.send(ServerMessage::String(s)).expect("can't send");
                                        }
                                        Message::Binary(b) => {
                                            println!("Got binary message from WebSocket {:?}", b);
                                            tx.send(ServerMessage::Binary(b)).expect("can't send");
                                        }
                                        Message::Close(_) => {
                                            println!("Got close message from WebSocket");
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
                        println!("WebSocket closed");
                    }
                    _ => {
                        println!("WebSocket is not open");
                    }
                }
            });
        });
    }
}

pub fn handle_tasks(mut commands: Commands, mut transform_tasks: Query<&mut ComputeTransform>) {
    for mut task in &mut transform_tasks {
        if let Some(mut commands_queue) = block_on(future::poll_once(&mut task.0)) {
            // append the returned command queue to have it execute later
            commands.append(&mut commands_queue);
        }
    }
}
