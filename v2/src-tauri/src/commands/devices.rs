pub use shield_optimizer_core::commands::devices::*;

#[tauri::command]
pub async fn ping_device(
    state: tauri::State<'_, super::AppState>,
    serial: String,
) -> Result<bool, String> {
    Ok(state
        .adb_snapshot()
        .await
        .shell(&serial, "true")
        .await
        .is_ok())
}
