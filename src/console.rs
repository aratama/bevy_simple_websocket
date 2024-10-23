// https://rustwasm.github.io/wasm-bindgen/examples/console-log.html

#[macro_export]
macro_rules! console_log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (web_sys::console::log_1(&JsValue::from(&format_args!($($t)*).to_string())))
}
