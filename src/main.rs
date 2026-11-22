// GranaFlow — gerenciador financeiro pessoal local-first.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ui;

use eframe::egui;
use granaflow::db::Database;

fn main() -> eframe::Result<()> {
    // GRANAFLOW_DB permite rodar com outro arquivo (demo, testes manuais).
    let path = std::env::var_os("GRANAFLOW_DB")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(Database::default_path);
    let db = match Database::open(&path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("GranaFlow: não foi possível abrir {}: {e}", path.display());
            std::process::exit(1);
        }
    };
    if let Err(e) = db.seed() {
        eprintln!("GranaFlow: falha ao preparar dados iniciais: {e}");
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([1000.0, 640.0])
            .with_title("GranaFlow"),
        ..Default::default()
    };

    let app_path = path.clone();
    eframe::run_native(
        "GranaFlow",
        options,
        Box::new(move |cc| Ok(Box::new(ui::App::new(cc, db, app_path)))),
    )
}
