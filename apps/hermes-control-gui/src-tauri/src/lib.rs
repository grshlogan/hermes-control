use hermes_control_gui::{
    GuiConfig, GuiConnectionSummary, GuiDaemonClient, GuiDashboardSnapshot, GuiLogTail,
    GuiLogTarget, gui_connection_summary_from_env,
};
use hermes_control_types::{HermesAction, ModelAction, OpenWebUiAction, OperationResponse, WslAction};

#[tauri::command]
fn gui_connection_summary() -> Result<GuiConnectionSummary, String> {
    gui_connection_summary_from_env().map_err(|err| err.to_string())
}

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
async fn gui_route_switch_execute(profile_id: String) -> Result<OperationResponse, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .route_switch_execute(profile_id, config.operator_id())
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
async fn gui_route_rollback_execute() -> Result<OperationResponse, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .route_rollback_execute(config.operator_id())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn gui_confirm_operation(
    code: String,
) -> Result<hermes_control_types::ConfirmationLifecycleResponse, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .confirm_operation(code, config.operator_id())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn gui_cancel_operation()
-> Result<hermes_control_types::ConfirmationLifecycleResponse, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .cancel_operation(config.operator_id())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn gui_model_action_preview(
    model_id: String,
    action: ModelAction,
) -> Result<OperationResponse, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .model_action_preview(model_id, action, config.operator_id())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn gui_model_action_execute(
    model_id: String,
    action: ModelAction,
) -> Result<OperationResponse, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .model_action_execute(model_id, action, config.operator_id())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn gui_wsl_action_preview(action: WslAction) -> Result<OperationResponse, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .wsl_action_preview(action, config.operator_id())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn gui_wsl_action_execute(action: WslAction) -> Result<OperationResponse, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .wsl_action_execute(action, config.operator_id())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn gui_hermes_action_preview(action: HermesAction) -> Result<OperationResponse, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .hermes_action_preview(action, config.operator_id())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn gui_hermes_action_execute(action: HermesAction) -> Result<OperationResponse, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .hermes_action_execute(action, config.operator_id())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn gui_openwebui_action_preview(action: OpenWebUiAction) -> Result<OperationResponse, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .openwebui_action_preview(action, config.operator_id())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn gui_openwebui_action_execute(action: OpenWebUiAction) -> Result<OperationResponse, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .openwebui_action_execute(action, config.operator_id())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn gui_provider_import_preview(payload: String) -> Result<hermes_control_types::ProviderImportPreviewResponse, String> {
    let config = GuiConfig::from_env().map_err(|err| err.to_string())?;
    let client = GuiDaemonClient::from_config(&config);
    client
        .provider_import_preview(payload, config.operator_id())
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
            gui_connection_summary,
            gui_dashboard_snapshot,
            gui_route_switch_preview,
            gui_route_switch_execute,
            gui_route_rollback_preview,
            gui_route_rollback_execute,
            gui_confirm_operation,
            gui_cancel_operation,
            gui_model_action_preview,
            gui_model_action_execute,
            gui_wsl_action_preview,
            gui_wsl_action_execute,
            gui_hermes_action_preview,
            gui_hermes_action_execute,
            gui_openwebui_action_preview,
            gui_openwebui_action_execute,
            gui_provider_import_preview,
            gui_log_tail
        ])
        .run(tauri::generate_context!())
        .expect("error while running Hermes Control GUI");
}
