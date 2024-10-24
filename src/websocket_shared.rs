use bevy::prelude::*;
use crossbeam_channel::Receiver;

#[derive(Resource, Deref)]
pub struct StreamReceiver(pub Receiver<ServerMessage>);

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
