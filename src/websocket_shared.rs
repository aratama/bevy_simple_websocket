use bevy::prelude::*;

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

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug)]
pub enum ReadyState {
    #[default]
    CONNECTING,
    OPEN,
    CLOSING,
    CLOSED,
}

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Resource)]
pub struct WebSocketState {
    pub ready_state: ReadyState,
}
