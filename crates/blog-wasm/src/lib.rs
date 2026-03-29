use wasm_bindgen::prelude::*;

pub mod api;
pub mod dom;
pub mod storage;

#[wasm_bindgen(start)]
pub fn start() {
    dom::render_app();
}
