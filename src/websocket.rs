// https://github.com/bevyengine/bevy/blob/main/examples/async_tasks/external_source_external_thread.rs

use bevy::prelude::*;
use crossbeam_channel::{bounded, Receiver};
use wasm_bindgen::prelude::*;
use web_sys::console;
use web_sys::Event;
use web_sys::MessageEvent;
use web_sys::WebSocket;

#[derive(Resource)]
struct WebSocketUrl(String);

// struct WebSocketSettings {
//     websocket: WebSocket
// }

#[derive(Default)]
pub struct WebSocketInstance {
    websocket: Option<WebSocket>,
    pub opened: bool,
}

#[derive(Resource, Deref)]
struct StreamReceiver(Receiver<WebSocketReader>);

#[derive(Event)]
pub enum WebSocketReader {
    Open,
    Message(String),
}

#[derive(Event)]
pub struct WebSocketWriter(pub String);

fn startup(
    mut commands: Commands,
    url: Res<WebSocketUrl>,
    mut instance: NonSendMut<WebSocketInstance>,
) {
    let (tx, rx) = bounded::<WebSocketReader>(10);

    let tx_ = tx.clone();
    let ws = WebSocket::new(&url.0).expect("Failed to create WebSocket");

    let on_open = Closure::<dyn Fn()>::new(move || {
        console::log_1(&"WebSocket opened".into());
        tx_.send(WebSocketReader::Open).unwrap();
    });
    ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    on_open.forget();

    let on_message = Closure::wrap(Box::new(move |event: Event| {
        let message_event = event
            .dyn_ref::<MessageEvent>()
            .expect("Event should be a MessageEvent");
        let message = message_event
            .data()
            .as_string()
            .expect("Data should be a string");
        console::log_1(&JsValue::from_str(
            format!("WebSocket message: {:?}", message).as_str(),
        ));

        tx.send(WebSocketReader::Message(message)).unwrap();
    }) as Box<dyn FnMut(_)>);
    ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();

    *instance = WebSocketInstance {
        websocket: Some(ws),
        opened: false,
    };

    commands.insert_resource(StreamReceiver(rx));
}

// This system reads from the receiver and sends events to Bevy
fn read_stream(
    receiver: Res<StreamReceiver>,
    mut events: EventWriter<WebSocketReader>,
    mut instance: NonSendMut<WebSocketInstance>,
) {
    for from_stream in receiver.try_iter() {
        match from_stream {
            WebSocketReader::Open => {
                instance.opened = true;
            }
            _ => {}
        }
        events.send(from_stream);
    }
}

fn write_message(mut events: EventReader<WebSocketWriter>, ws_nonsend: NonSend<WebSocketInstance>) {
    if let Some(ws) = &ws_nonsend.websocket {
        for event in events.read() {
            ws.send_with_str(&event.0).unwrap();
        }
    }
}

pub struct WebSocketPlugin {
    pub url: String,
}

impl Plugin for WebSocketPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup)
            .add_systems(Update, (read_stream, write_message))
            .add_event::<WebSocketReader>()
            .add_event::<WebSocketWriter>()
            .insert_resource(WebSocketUrl(self.url.clone()))
            .init_non_send_resource::<WebSocketInstance>();
    }
}
