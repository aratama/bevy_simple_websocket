# bevy_simple_websocket

Extremely simple WebSocket client library for Bevy.
This library works in both native and WASM(Web Browser).

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

https://aratama.github.io/bevy_simple_websocket/

# References

- https://github.com/bevyengine/bevy/blob/main/examples/async_tasks/external_source_external_thread.rs
