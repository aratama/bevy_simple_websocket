use crate::websocket_shared::*;
use bevy::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::websocket_wasm::*;

pub struct WebSocketPlugin;

impl Plugin for WebSocketPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ServerMessage>()
            .add_event::<ClientMessage>()
            .init_non_send_resource::<WebSocketInstance>();

        #[cfg(target_arch = "wasm32")]
        app.add_systems(Update, (read_stream_wasm, write_message_wasm));
    }
}
