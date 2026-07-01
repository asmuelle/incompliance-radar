//! Thin wasm entry point. Kept separate from the `app` crate so the shared
//! UI/domain code stays a plain rlib and only this crate needs to be built
//! as a `cdylib` for `wasm32-unknown-unknown`.

use app::App;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
