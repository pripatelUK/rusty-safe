//! Rusty-Safe: A Rust-native Safe{Wallet} transaction verification GUI

use eframe::egui;

mod app;

fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting Rusty-Safe");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Rusty-Safe")
            .with_inner_size([900.0, 700.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Rusty-Safe",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
