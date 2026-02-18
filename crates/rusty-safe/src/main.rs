//! Rusty-Safe: A Rust-native Safe{Wallet} transaction verification GUI
//!
//! Uses safe-utils from safe-hash-rs for all hash computation and chain data.

mod api;
mod app;
mod decode;
mod expected;
mod hasher;
mod sidebar;
mod signing_bridge;
mod signing_ui;
mod state;
mod ui;

// Web entry point
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    tracing_wasm::set_as_global_default();
    tracing::info!("Starting Rusty-Safe (WASM)");

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("Element is not a canvas");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
            )
            .await;

        if let Some(loading) = document.get_element_by_id("loading") {
            loading.remove();
        }

        if let Err(e) = start_result {
            tracing::error!("Failed to start eframe: {:?}", e);
        }
    });
}

// Native entry point
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting Rusty-Safe");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Rusty Safe")
            .with_inner_size([900.0, 700.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Rusty Safe",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
