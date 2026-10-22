mod store;
mod excel;
mod backup;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![store::load_state, store::save_state, backup::backup_now, backup::backup_status, backup::export_excel_dialog, backup::choose_excel_import])
        .run(tauri::generate_context!())
        .expect("No se pudo iniciar AutoTrace Desktop");
}
