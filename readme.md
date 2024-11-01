# bevy_simple_websocket

Simple WebSocket client library for [Bevy Engine](https://bevyengine.org/).

This is a "raw" WebSocket client library, so you can send to and receive from servers written in any programming language.

This library works in both native and WASM.

## Basic Usage

### Establishing a connection

You can request opening connection with sending a `ClientMessage::Open` event to `EventWriter<ClientMessage>`.

```rust
fn setup_system(
    ...
    mut writer: EventWriter<ClientMessage>,
) {
    ...
    writer.send(ClientMessage::Open(url));
}
```

`url` is a URL of WebSocket server like `wss://some-websocket-server.example.com`.

### Sending Messages

After a connection established, the client can start sending text messages with `ClientMessage::String(str)` event. You should check the current state via `Res<WebSocketState>`.

```rust
fn send_message_system(
    ...
    state: Res<WebSocketState>,
    mut writer: EventWriter<ClientMessage>,
) {
    if state.ready_state == ReadyState::OPEN {
        ...
        writer.send(ClientMessage::String(str));
        ...
    }
}
```

You can also send binary messages as `writer.send(ClientMessage::Binary(bin))`.

### Receiving Messages

Messages from the server can be received through the `EventReader<ServerMessage>` event.

```rust
fn process_message_system(
    ...
    mut reader: EventReader<ServerMessage>,
) {
    for event in reader.read() {
        match event {
            ServerMessage::Open(message) => { ... }
            ServerMessage::String(message) => { ... }
            ServerMessage::Binary(bytes) => { ... }
            ServerMessage::Close => { ... }
        }
    }
}
```
