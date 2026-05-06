import { describe, expect, it } from 'vitest';
import {
  buildLogTargets,
  buildRouteOptions,
  buildDashboardViewModel,
  navigationSections,
  unsafeTauriPermissionPrefixes,
} from './viewModel';
import type { GuiDashboardSnapshot } from './types';

const snapshot: GuiDashboardSnapshot = {
  status: {
    wsl: { name: 'Ubuntu-Hermes-Codex', state: 'Running', version: 2, default: true },
    hermes: {
      url: 'http://127.0.0.1:8642/health',
      reachable: true,
      status_code: 200,
      message: 'ok',
    },
    models: [
      {
        runtime_id: 'qwen36',
        variant_id: 'qwen36-mtp',
        served_model_name: 'qwen36-mtp',
        endpoint: {
          url: 'http://10.2.176.55:18080/v1/models',
          reachable: true,
          status_code: 200,
          message: 'ok',
        },
        ready: true,
      },
    ],
    state: { state_db_exists: true, audit_db_exists: true },
    overall: 'Ok',
  },
  active_route: {
    active_profile_id: 'local.qwen36-mtp',
    last_known_good_profile_id: 'external.deepseek',
  },
  providers: [
    {
      id: 'local.qwen36-mtp',
      kind: 'LocalVllm',
      display_name: 'Qwen 36 MTP',
      models: ['qwen36-mtp'],
    },
    {
      id: 'external.deepseek',
      kind: 'DeepSeek',
      display_name: 'DeepSeek',
      models: ['deepseek-chat'],
    },
  ],
  models: [
    {
      runtime_id: 'qwen36',
      variant_id: 'qwen36-mtp',
      served_model_name: 'qwen36-mtp',
      endpoint: {
        url: 'http://10.2.176.55:18080/v1/models',
        reachable: true,
        status_code: 200,
        message: 'ok',
      },
      ready: true,
    },
  ],
  audit: [
    {
      id: 1,
      happened_at: '2026-05-06T10:00:00Z',
      requester_channel: 'gui',
      requester_user_id: 'desktop-operator',
      action: 'route.switch',
      risk_level: 'NormalMutating',
      summary: 'Switched route',
    },
  ],
};

describe('Phase8 GUI view model', () => {
  it('builds an operations-first dashboard summary from daemon state', () => {
    const model = buildDashboardViewModel(snapshot);

    expect(model.overallLabel).toBe('OK');
    expect(model.activeRoute).toBe('local.qwen36-mtp');
    expect(model.lastKnownGoodRoute).toBe('external.deepseek');
    expect(model.readyModels).toBe(1);
    expect(model.totalModels).toBe(1);
    expect(model.wslState).toBe('Running');
    expect(model.hermesReachable).toBe(true);
  });

  it('keeps Phase8 navigation focused on control surfaces', () => {
    expect(navigationSections.map((section) => section.id)).toEqual([
      'dashboard',
      'route',
      'models',
      'runtime',
      'logs',
      'audit',
      'settings',
    ]);
  });

  it('documents that broad Tauri permissions stay out of the GUI app', () => {
    expect(unsafeTauriPermissionPrefixes).toEqual(['shell:', 'fs:', 'process']);
  });

  it('marks route options with active and last-known-good context', () => {
    expect(buildRouteOptions(snapshot)).toEqual([
      {
        id: 'local.qwen36-mtp',
        label: 'Qwen 36 MTP',
        kind: 'LocalVllm',
        isActive: true,
        isLastKnownGood: false,
      },
      {
        id: 'external.deepseek',
        label: 'DeepSeek',
        kind: 'DeepSeek',
        isActive: false,
        isLastKnownGood: true,
      },
    ]);
  });

  it('keeps log target selection bounded to daemon-owned safe targets', () => {
    expect(buildLogTargets()).toEqual([
      { id: 'daemon', label: 'Daemon' },
      { id: 'bot', label: 'Bot' },
      { id: 'hermes', label: 'Hermes' },
    ]);
  });
});
