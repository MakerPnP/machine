#![warn(clippy::all, rust_2018_idioms)]
// In release mode *without* console_logging → use GUI subsystem (hide console)
// In debug mode → console always enabled
// In release mode *with* console_logging → console is enabled
#![cfg_attr(
    all(not(debug_assertions), not(feature = "console_logging")),
    windows_subsystem = "windows"
)]

use egui_i18n::tr;
use i18n::I18nConfig;
use operator_ui_egui::LOGO;
/// Run as follows:
/// `run --package planner_gui_egui --bin planner_gui_egui`
///
/// To enable logging, set the environment variable appropriately, for example:
/// `RUST_LOG=debug,eframe=warn,egui_glow=warn,egui=warn`
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    early_init();

    let default_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(std::sync::Arc::new(
                eframe::icon_data::from_png_bytes(&LOGO[..]).expect("Failed to load icon"),
            ))
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    let app_name = tr!("main-window-title");

    if let Err(e) = eframe::run_native(
        &app_name,
        default_options.clone(),
        Box::new(|cc| Ok(Box::new(operator_ui_egui::OperatorUiApp::new(cc)))),
    ) {
        eprintln!(
            "Failed to run eframe: {:?}, trying with hardware acceleration disabled.",
            e
        );

        // Fallback: force software renderer
        let mut sw_options = default_options.clone();
        sw_options.hardware_acceleration = eframe::HardwareAcceleration::Off;

        if let Err(e) = eframe::run_native(
            &app_name,
            default_options.clone(),
            Box::new(|cc| Ok(Box::new(operator_ui_egui::OperatorUiApp::new(cc)))),
        ) {
            eprintln!(
                "Failed to run eframe with hardware acceleration disabled, cause: {:?}",
                e
            );

            return Err(e);
        }
    }

    Ok(())
}

fn early_init() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    info!("Started");

    operator_ui_egui::profiling::init();

    i18n::init(I18nConfig {
        languages: vec![String::from("es-ES"), String::from("en-US")],
        default: "en-US".to_string(),
        fallback: "en-US".to_string(),
    });
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    early_init();

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(operator_ui_egui::OperatorUiApp::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html("<p> The app has crashed. See the developer console for details. </p>");
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
