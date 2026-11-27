//! Nephron — Local-first Rust GUI workspace on top of Cynapse & Dendrite.

mod app;
mod brain;
mod doctor;
mod overlays;
mod theme;

use app::NephronApp;
use nephron_bridge::{client::DEFAULT_ENDPOINT, CynapseClient, MemoryStore};
use std::path::PathBuf;

fn default_db_path() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("nephron").join("dendrite.db")
}

fn main() -> eframe::Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");

    let endpoint = std::env::var("NEPHRON_CYNAPSE_URL")
        .unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string());
    match std::env::args().nth(1).as_deref() {
        Some("doctor") => {
            rt.block_on(doctor::run_doctor(&endpoint));
            return Ok(());
        }
        Some("--pet-only") => {
            let sp = std::env::args()
                .nth(2)
                .map(|s| nephron_pets::PetSpecies::from_arg(&s))
                .unwrap_or(nephron_pets::PetSpecies::Chiba);
            let kind = match std::env::args().nth(3).as_deref() {
                Some("ollama") => nephron_bridge::BackendKind::Ollama,
                _ => nephron_bridge::BackendKind::Cynapse,
            };
            let pet_url = std::env::args()
                .nth(4)
                .unwrap_or_else(|| kind.default_url().to_string());
            let pet_model = std::env::args()
                .nth(5)
                .unwrap_or_else(|| nephron_bridge::ModelProfile::MiniCpm5_1B.id().to_string());
            let client = CynapseClient::with_kind(pet_url, kind).expect("valid pet endpoint");
            return overlays::run_pet_only(rt, client, sp, pet_model);
        }
        Some("--swarm-only") => {
            return overlays::run_swarm_only();
        }
        _ => {}
    }
    let client = CynapseClient::new(endpoint).expect("valid cynapse endpoint");
    let store = MemoryStore::open(default_db_path())
        .or_else(|_| MemoryStore::open_in_memory())
        .expect("dendrite sqlite store");

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_title("Nephron"),
        ..Default::default()
    };

    eframe::run_native(
        "Nephron",
        native_options,
        Box::new(move |cc| {
            theme::apply_hud_theme(&cc.egui_ctx);
            Ok(Box::new(NephronApp::new(rt, client, store)))
        }),
    )
}
