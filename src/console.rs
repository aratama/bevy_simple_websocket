// https://rustwasm.github.io/wasm-bindgen/examples/console-log.html

#[macro_export]
macro_rules! console_log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (web_sys::console::log_1(&JsValue::from(&format_args!($($t)*).to_string())))
}

#[macro_export]
macro_rules! console_debug {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (web_sys::console::debug_1(&JsValue::from(&format_args!($($t)*).to_string())))
}

#[macro_export]
macro_rules! console_error {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (web_sys::console::error_1(&JsValue::from(&format_args!($($t)*).to_string())))
}
