mod console;
mod ex;
pub mod websocket_shared;

pub use crate::websocket_shared::*;
use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksPlugin;

#[cfg(target_arch = "wasm32")]
mod websocket_wasm;

#[cfg(not(target_arch = "wasm32"))]
mod websocket_native;

#[cfg(target_arch = "wasm32")]
use crate::websocket_wasm::*;

#[cfg(not(target_arch = "wasm32"))]
use crate::websocket_native::*;

#[cfg(target_arch = "wasm32")]
pub use websocket_wasm::WebSocketInstance;

#[cfg(not(target_arch = "wasm32"))]
pub use websocket_native::WebSocketInstance;

pub struct WebSocketPlugin;

impl Plugin for WebSocketPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TokioTasksPlugin::default())
            .add_event::<ServerMessage>()
            .add_event::<ClientMessage>()
            .init_non_send_resource::<WebSocketInstance>();

        #[cfg(target_arch = "wasm32")]
        app.add_systems(Update, (read_stream_wasm, write_message_wasm));

        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, (read_stream_native, write_message_native));
    }
}
