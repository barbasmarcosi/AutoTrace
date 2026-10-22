mod store;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![store::load_state, store::save_state])
        .run(tauri::generate_context!())
        .expect("No se pudo iniciar AutoTrace Desktop");
}
