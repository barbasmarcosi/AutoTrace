mod store;
mod excel;
mod backup;
mod client_report;
mod legacy_documents;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![store::load_state, store::save_state, backup::backup_now, backup::backup_status, backup::export_excel_dialog, backup::choose_excel_import, backup::export_customer_excel_dialog, backup::choose_sqlite_import, backup::configure_external_backup, backup::test_backup_recovery, legacy_documents::choose_legacy_document, legacy_documents::export_legacy_document])
        .run(tauri::generate_context!())
        .expect("No se pudo iniciar AutoTrace Desktop");
}
