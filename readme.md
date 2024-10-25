# bevy_websocket_sync

Simple WebSocket client library for Bevy.
This library works in both native and WASM.

## Usage of Example

Add `.env` as:

```
url=ws://localhost:8080
```

Then, start WebSocket server:

```
$ cd serve
$ node server
```

Finally, start client:

```
$ trunk serve
```

# Demo

https://aratama.github.io/bevy_websocket_sync/

# References

- https://github.com/bevyengine/bevy/blob/main/examples/async_tasks/external_source_external_thread.rs
