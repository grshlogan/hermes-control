import { invoke } from '@tauri-apps/api/core';
import type { GuiDashboardSnapshot, GuiLogTail, OperationResponse } from './types';

interface BrowserGuiSettings {
  daemonUrl: string;
  apiToken: string;
  operatorId: string;
}

const SETTINGS_KEY = 'hermes-control-gui-settings';

export function readBrowserSettings(): BrowserGuiSettings {
  if (typeof window === 'undefined') {
    return { daemonUrl: 'http://127.0.0.1:18787', apiToken: '', operatorId: 'browser-gui' };
  }

  const stored = window.localStorage.getItem(SETTINGS_KEY);
  if (!stored) {
    return { daemonUrl: 'http://127.0.0.1:18787', apiToken: '', operatorId: 'browser-gui' };
  }

  try {
    const parsed = JSON.parse(stored) as Partial<BrowserGuiSettings>;
    return {
      daemonUrl: parsed.daemonUrl || 'http://127.0.0.1:18787',
      apiToken: parsed.apiToken || '',
      operatorId: parsed.operatorId || 'browser-gui',
    };
  } catch {
    return { daemonUrl: 'http://127.0.0.1:18787', apiToken: '', operatorId: 'browser-gui' };
  }
}

export function saveBrowserSettings(settings: BrowserGuiSettings): void {
  window.localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
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

export async function loadLogTail(target: 'daemon' | 'bot' | 'hermes'): Promise<GuiLogTail> {
  if (isTauriRuntime()) {
    return invoke<GuiLogTail>('gui_log_tail', { target, tail: 200 });
  }

  return getJson(readBrowserSettings(), `/v1/logs/${target}?tail=200`) as Promise<GuiLogTail>;
}

async function loadDashboardSnapshotViaHttp(
  settings: BrowserGuiSettings,
): Promise<GuiDashboardSnapshot> {
  if (!settings.apiToken) {
    throw new Error('Set HERMES_CONTROL_API_TOKEN in the desktop app environment or browser settings.');
  }

  const [status, activeRoute, providers, models, audit] = await Promise.all([
    getJson(settings, '/v1/status'),
    getJson(settings, '/v1/route/active'),
    getJson(settings, '/v1/providers'),
    getJson(settings, '/v1/models'),
    getJson(settings, '/v1/audit?limit=20'),
  ]);

  return {
    status,
    active_route: activeRoute,
    providers,
    models,
    audit,
  } as GuiDashboardSnapshot;
}

async function getJson(settings: BrowserGuiSettings, path: string): Promise<unknown> {
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
  settings: BrowserGuiSettings,
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

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
