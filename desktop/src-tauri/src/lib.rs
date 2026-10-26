mod store;
mod excel;
mod backup;
mod drive_backup;
mod drive_oauth;
mod drive_transport;
mod drive_api;
mod client_report;
mod customer_pdf;
mod legacy_documents;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|_app|{
            // Best effort only: never block opening the workshop because Windows
            // refuses to remove one stale temporary recovery directory.
            drive_backup::cleanup_stale_decrypt_directories();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![store::load_state, store::save_state, backup::backup_now, backup::backup_status, backup::export_excel_dialog, backup::choose_excel_import, backup::export_customer_excel_dialog, backup::choose_sqlite_import, backup::configure_external_backup, backup::test_backup_recovery, drive_backup::configure_drive_backup, drive_backup::unlock_drive_backup, drive_backup::lock_drive_backup, drive_backup::drive_backup_status, drive_backup::choose_drive_restore, drive_backup::test_drive_recovery, drive_backup::generate_recovery_phrase, drive_backup::export_portable_encrypted_backup, drive_api::drive_api_status, drive_api::drive_api_set_client, drive_api::drive_api_connect, drive_api::drive_api_disconnect, drive_api::drive_api_unlock, drive_api::drive_api_enable_auto_unlock, drive_api::drive_api_disable_auto_unlock, drive_api::drive_api_list, drive_api::drive_api_retry, drive_api::drive_api_test, drive_api::drive_api_preview, legacy_documents::choose_legacy_document, legacy_documents::export_legacy_document, legacy_documents::choose_workshop_logo, customer_pdf::save_customer_pdf])
        .run(tauri::generate_context!())
        .expect("No se pudo iniciar AutoTrace Desktop");
}
