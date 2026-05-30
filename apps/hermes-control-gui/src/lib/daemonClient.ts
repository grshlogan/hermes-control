import { invoke } from '@tauri-apps/api/core';
import type {
  ConfirmationLifecycleResponse,
  GuiConnectionSettings,
  GuiConnectionSummary,
  GuiDashboardSnapshot,
  GuiLogTail,
  HermesActionId,
  LogTargetId,
  ModelActionId,
  OpenWebUiActionId,
  OperationResponse,
  ProviderImportPreviewResponse,
  WslActionId,
} from './types';

const SETTINGS_KEY = 'hermes-control-gui-settings';

export function readBrowserSettings(): GuiConnectionSettings {
  if (typeof window === 'undefined') {
    return { daemonUrl: 'http://127.0.0.1:18787', apiToken: '', operatorId: 'browser-gui' };
  }

  const stored = window.localStorage.getItem(SETTINGS_KEY);
  if (!stored) {
    return { daemonUrl: 'http://127.0.0.1:18787', apiToken: '', operatorId: 'browser-gui' };
  }

  try {
    const parsed = JSON.parse(stored) as Partial<GuiConnectionSettings>;
    return {
      daemonUrl: parsed.daemonUrl || 'http://127.0.0.1:18787',
      apiToken: parsed.apiToken || '',
      operatorId: parsed.operatorId || 'browser-gui',
    };
  } catch {
    return { daemonUrl: 'http://127.0.0.1:18787', apiToken: '', operatorId: 'browser-gui' };
  }
}

export function saveBrowserSettings(settings: GuiConnectionSettings): void {
  window.localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
}

export function isDesktopRuntime(): boolean {
  return isTauriRuntime();
}

export async function loadConnectionSummary(): Promise<GuiConnectionSettings> {
  if (isTauriRuntime()) {
    const summary = await invoke<GuiConnectionSummary>('gui_connection_summary');
    return {
      daemonUrl: summary.daemon_url,
      apiToken: summary.token_configured ? summary.token_label : '',
      operatorId: summary.operator_id,
    };
  }

  return readBrowserSettings();
}

export async function loadDashboardSnapshot(): Promise<GuiDashboardSnapshot> {
  if (isTauriRuntime()) {
    return invoke<GuiDashboardSnapshot>('gui_dashboard_snapshot');
  }

  return loadDashboardSnapshotViaHttp(readBrowserSettings());
}

export async function previewRouteSwitch(profileId: string): Promise<OperationResponse> {
  if (isTauriRuntime()) {
    return invoke<OperationResponse>('gui_route_switch_preview', { profileId });
  }

  const settings = readBrowserSettings();
  return postJson(settings, '/v1/route/switch', {
    requester: guiRequester(settings.operatorId),
    profile_id: profileId,
    reason: `GUI route switch ${profileId}`,
    dry_run: true,
  }) as Promise<OperationResponse>;
}

export async function executeRouteSwitch(profileId: string): Promise<OperationResponse> {
  if (isTauriRuntime()) {
    return invoke<OperationResponse>('gui_route_switch_execute', { profileId });
  }

  const settings = readBrowserSettings();
  return postJson(settings, '/v1/route/switch', {
    requester: guiRequester(settings.operatorId),
    profile_id: profileId,
    reason: `GUI route switch ${profileId}`,
    dry_run: false,
  }) as Promise<OperationResponse>;
}

export async function previewRouteRollback(): Promise<OperationResponse> {
  if (isTauriRuntime()) {
    return invoke<OperationResponse>('gui_route_rollback_preview');
  }

  const settings = readBrowserSettings();
  return postJson(settings, '/v1/route/rollback', {
    requester: guiRequester(settings.operatorId),
    reason: 'GUI route rollback',
    dry_run: true,
  }) as Promise<OperationResponse>;
}

export async function executeRouteRollback(): Promise<OperationResponse> {
  if (isTauriRuntime()) {
    return invoke<OperationResponse>('gui_route_rollback_execute');
  }

  const settings = readBrowserSettings();
  return postJson(settings, '/v1/route/rollback', {
    requester: guiRequester(settings.operatorId),
    reason: 'GUI route rollback',
    dry_run: false,
  }) as Promise<OperationResponse>;
}

export async function confirmOperation(code: string): Promise<ConfirmationLifecycleResponse> {
  if (isTauriRuntime()) {
    return invoke<ConfirmationLifecycleResponse>('gui_confirm_operation', { code });
  }

  const settings = readBrowserSettings();
  return postJson(settings, '/v1/confirm', {
    requester: guiRequester(settings.operatorId),
    code,
  }) as Promise<ConfirmationLifecycleResponse>;
}

export async function cancelOperation(): Promise<ConfirmationLifecycleResponse> {
  if (isTauriRuntime()) {
    return invoke<ConfirmationLifecycleResponse>('gui_cancel_operation');
  }

  const settings = readBrowserSettings();
  return postJson(settings, '/v1/cancel', {
    requester: guiRequester(settings.operatorId),
  }) as Promise<ConfirmationLifecycleResponse>;
}

export async function previewModelAction(
  modelId: string,
  action: ModelActionId,
): Promise<OperationResponse> {
  if (isTauriRuntime()) {
    return invoke<OperationResponse>('gui_model_action_preview', { modelId, action });
  }

  const settings = readBrowserSettings();
  return postJson(settings, `/v1/models/${modelId}/action`, {
    requester: guiRequester(settings.operatorId),
    action,
    reason: `GUI model ${action.toLowerCase()} ${modelId}`,
    dry_run: true,
  }) as Promise<OperationResponse>;
}

export async function executeModelAction(
  modelId: string,
  action: ModelActionId,
): Promise<OperationResponse> {
  if (isTauriRuntime()) {
    return invoke<OperationResponse>('gui_model_action_execute', { modelId, action });
  }

  const settings = readBrowserSettings();
  return postJson(settings, `/v1/models/${modelId}/action`, {
    requester: guiRequester(settings.operatorId),
    action,
    reason: `GUI model ${action.toLowerCase()} ${modelId}`,
    dry_run: false,
  }) as Promise<OperationResponse>;
}

export async function previewWslAction(action: WslActionId): Promise<OperationResponse> {
  if (isTauriRuntime()) {
    return invoke<OperationResponse>('gui_wsl_action_preview', { action });
  }

  const settings = readBrowserSettings();
  return postJson(settings, '/v1/wsl/action', {
    requester: guiRequester(settings.operatorId),
    action,
    reason: `GUI WSL ${runtimeActionReason(action)}`,
    dry_run: true,
  }) as Promise<OperationResponse>;
}

export async function executeWslAction(action: WslActionId): Promise<OperationResponse> {
  if (isTauriRuntime()) {
    return invoke<OperationResponse>('gui_wsl_action_execute', { action });
  }

  const settings = readBrowserSettings();
  return postJson(settings, '/v1/wsl/action', {
    requester: guiRequester(settings.operatorId),
    action,
    reason: `GUI WSL ${runtimeActionReason(action)}`,
    dry_run: false,
  }) as Promise<OperationResponse>;
}

export async function previewHermesAction(action: HermesActionId): Promise<OperationResponse> {
  if (isTauriRuntime()) {
    return invoke<OperationResponse>('gui_hermes_action_preview', { action });
  }

  const settings = readBrowserSettings();
  return postJson(settings, '/v1/hermes/action', {
    requester: guiRequester(settings.operatorId),
    action,
    reason: `GUI Hermes ${runtimeActionReason(action)}`,
    dry_run: true,
  }) as Promise<OperationResponse>;
}

export async function executeHermesAction(action: HermesActionId): Promise<OperationResponse> {
  if (isTauriRuntime()) {
    return invoke<OperationResponse>('gui_hermes_action_execute', { action });
  }

  const settings = readBrowserSettings();
  return postJson(settings, '/v1/hermes/action', {
    requester: guiRequester(settings.operatorId),
    action,
    reason: `GUI Hermes ${runtimeActionReason(action)}`,
    dry_run: false,
  }) as Promise<OperationResponse>;
}

export async function previewOpenWebUiAction(action: OpenWebUiActionId): Promise<OperationResponse> {
  if (isTauriRuntime()) {
    return invoke<OperationResponse>('gui_openwebui_action_preview', { action });
  }

  const settings = readBrowserSettings();
  return postJson(settings, '/v1/openwebui/action', {
    requester: guiRequester(settings.operatorId),
    action,
    reason: `GUI Open WebUI ${runtimeActionReason(action)}`,
    dry_run: true,
  }) as Promise<OperationResponse>;
}

export async function executeOpenWebUiAction(action: OpenWebUiActionId): Promise<OperationResponse> {
  if (isTauriRuntime()) {
    return invoke<OperationResponse>('gui_openwebui_action_execute', { action });
  }

  const settings = readBrowserSettings();
  return postJson(settings, '/v1/openwebui/action', {
    requester: guiRequester(settings.operatorId),
    action,
    reason: `GUI Open WebUI ${runtimeActionReason(action)}`,
    dry_run: false,
  }) as Promise<OperationResponse>;
}

export async function previewProviderImport(payload: string): Promise<ProviderImportPreviewResponse> {
  if (isTauriRuntime()) {
    return invoke<ProviderImportPreviewResponse>('gui_provider_import_preview', { payload });
  }

  const settings = readBrowserSettings();
  return postJson(settings, '/v1/providers/import/preview', {
    requester: guiRequester(settings.operatorId),
    source: 'json',
    payload,
    dry_run: true,
  }) as Promise<ProviderImportPreviewResponse>;
}

export async function loadLogTail(
  target: LogTargetId,
  tail = 200,
): Promise<GuiLogTail> {
  const safeTail = Math.max(1, Math.min(1000, Math.trunc(tail)));

  if (isTauriRuntime()) {
    return invoke<GuiLogTail>('gui_log_tail', { target, tail: safeTail });
  }

  return getJson(readBrowserSettings(), `/v1/logs/${target}?tail=${safeTail}`) as Promise<GuiLogTail>;
}

async function loadDashboardSnapshotViaHttp(
  settings: GuiConnectionSettings,
): Promise<GuiDashboardSnapshot> {
  if (!settings.apiToken) {
    throw new Error('Set HERMES_CONTROL_API_TOKEN in the desktop app environment or browser settings.');
  }

  const [status, activeRoute, providers, audit] = await Promise.all([
    getJson(settings, '/v1/status'),
    getJson(settings, '/v1/route/active'),
    getJson(settings, '/v1/providers'),
    getJson(settings, '/v1/audit?limit=20'),
  ]);

  return {
    status,
    active_route: activeRoute,
    providers,
    models: (status as GuiDashboardSnapshot['status']).models,
    audit,
  } as GuiDashboardSnapshot;
}

async function getJson(settings: GuiConnectionSettings, path: string): Promise<unknown> {
  const url = new URL(path, settings.daemonUrl);
  const response = await fetch(url, {
    headers: {
      Authorization: `Bearer ${settings.apiToken}`,
    },
  });
  if (!response.ok) {
    throw new Error(`Daemon request failed: ${response.status} ${response.statusText}`);
  }
  return response.json();
}

async function postJson(
  settings: GuiConnectionSettings,
  path: string,
  body: unknown,
): Promise<unknown> {
  if (!settings.apiToken) {
    throw new Error('Set HERMES_CONTROL_API_TOKEN in the desktop app environment or browser settings.');
  }

  const url = new URL(path, settings.daemonUrl);
  const response = await fetch(url, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${settings.apiToken}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    throw new Error(`Daemon request failed: ${response.status} ${response.statusText}`);
  }
  return response.json();
}

function guiRequester(operatorId: string) {
  return {
    channel: 'gui',
    user_id: operatorId,
    chat_id: null,
  };
}

function runtimeActionReason(action: string): string {
  return action.replace(/([a-z])([A-Z])/g, '$1 $2').toLowerCase();
}

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
