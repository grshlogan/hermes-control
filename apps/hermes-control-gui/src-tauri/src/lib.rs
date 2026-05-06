use hermes_control_gui::{
    GuiConfig, GuiDaemonClient, GuiDashboardSnapshot, GuiLogTail, GuiLogTarget,
};
use hermes_control_types::OperationResponse;

#[tauri::command]
async fn gui_dashboard_snapshot() -> Result<GuiDashboardSnapshot, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .dashboard_snapshot()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn gui_route_switch_preview(profile_id: String) -> Result<OperationResponse, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .route_switch_preview(profile_id, config.operator_id())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn gui_route_rollback_preview() -> Result<OperationResponse, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .route_rollback_preview(config.operator_id())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn gui_log_tail(target: GuiLogTarget, tail: usize) -> Result<GuiLogTail, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .tail_logs(target, tail)
        .await
        .map_err(|err| err.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            gui_dashboard_snapshot,
            gui_route_switch_preview,
            gui_route_rollback_preview,
            gui_log_tail
        ])
        .run(tauri::generate_context!())
        .expect("error while running Hermes Control GUI");
}
