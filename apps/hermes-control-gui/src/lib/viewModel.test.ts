import { describe, expect, it } from 'vitest';
import {
  buildConfirmationPrompt,
  buildAuditRiskOptions,
  buildDashboardViewModel,
  buildFilteredAuditEvents,
  buildFilteredLogLines,
  buildLogTargets,
  buildHermesActionOptions,
  buildModelActionProgressMessage,
  buildModelActionOptions,
  buildRouteOptions,
  buildSettingsViewModel,
  buildStateStoreSummary,
  buildWslActionOptions,
  navigationSections,
  translateAuditRiskFilter,
  translateRiskLevel,
  unsafeTauriPermissionPrefixes,
} from './viewModel';
import { createTranslator } from './i18n';
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
        model_root: '/root/Hermres/models',
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
      model_root: '/root/Hermres/models',
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

  it('carries the WSL-native model root into model summaries for GUI display', () => {
    expect(snapshot.models[0].model_root).toBe('/root/Hermres/models');
  });

  it('summarizes state and audit databases together for dashboard detail', () => {
    const t = createTranslator('zh-CN');

    expect(buildStateStoreSummary(snapshot.status.state, t)).toBe('状态库：存在 / 审计库：存在');
    expect(
      buildStateStoreSummary(
        {
          state_db_exists: true,
          audit_db_exists: false,
        },
        t,
      ),
    ).toBe('状态库：存在 / 审计库：缺失');
    expect(buildStateStoreSummary(null, t)).toBe('未知');
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
      { id: 'vllm', label: 'vLLM' },
    ]);
  });

  it('exposes model actions expected by bot and CLI without raw shell controls', () => {
    expect(buildModelActionOptions()).toEqual([
      { id: 'Install', label: 'Install', riskHint: 'Normal' },
      { id: 'Start', label: 'Start', riskHint: 'Normal' },
      { id: 'Stop', label: 'Stop', riskHint: 'Destructive' },
      { id: 'Restart', label: 'Restart', riskHint: 'Destructive' },
      { id: 'Health', label: 'Health', riskHint: 'Read-only' },
      { id: 'Logs', label: 'Logs', riskHint: 'Read-only' },
      { id: 'Benchmark', label: 'Benchmark', riskHint: 'Experimental' },
    ]);
  });

  it('explains long-running local model startup instead of showing a raw action id', () => {
    const zh = createTranslator('zh-CN');
    const en = createTranslator('en-US');

    expect(buildModelActionProgressMessage('qwen36-mtp', 'Start', zh)).toEqual({
      message: '正在启动模型：qwen36-mtp。vLLM/MTP 模型加载可能需要几分钟。可切到日志页查看 vLLM 输出。',
      longRunning: true,
    });
    expect(buildModelActionProgressMessage('qwen36-mtp', 'Restart', en)).toEqual({
      message: 'Restarting model: qwen36-mtp. vLLM/MTP model loading can take several minutes. Check Logs for vLLM output.',
      longRunning: true,
    });
    expect(buildModelActionProgressMessage('qwen36-mtp', 'Health', zh)).toEqual({
      message: '正在提交模型操作：qwen36-mtp / 健康检查',
      longRunning: false,
    });
  });

  it('exposes runtime actions as typed daemon options', () => {
    expect(buildWslActionOptions()).toEqual([
      { id: 'Wake', label: 'Wake', riskHint: 'Normal' },
      { id: 'StopDistro', label: 'Stop', riskHint: 'Destructive' },
      { id: 'RestartDistro', label: 'Restart', riskHint: 'Destructive' },
      { id: 'ShutdownAll', label: 'Shutdown all', riskHint: 'Destructive' },
    ]);
    expect(buildHermesActionOptions()).toEqual([
      { id: 'Wake', label: 'Wake', riskHint: 'Normal' },
      { id: 'Stop', label: 'Stop', riskHint: 'Destructive' },
      { id: 'Restart', label: 'Restart', riskHint: 'Destructive' },
      { id: 'Kill', label: 'Kill', riskHint: 'Destructive' },
    ]);
  });

  it('builds a confirmation prompt only when the daemon requires one', () => {
    expect(
      buildConfirmationPrompt({
        status: 'completed',
        risk: 'NormalMutating',
        summary: 'Switched route',
        dry_run: false,
      }),
    ).toBeNull();

    expect(
      buildConfirmationPrompt({
        status: 'confirmation_required',
        risk: 'Destructive',
        summary: 'Restart WSL distro Ubuntu-Hermes-Codex',
        dry_run: false,
        confirmation_id: 'confirm_1',
        code_hint: 'HERMES-7421',
        expires_at: '2026-05-06T10:05:00Z',
      }),
    ).toEqual({
      confirmationId: 'confirm_1',
      codeHint: 'HERMES-7421',
      expiresAt: '2026-05-06T10:05:00Z',
      summary: 'Restart WSL distro Ubuntu-Hermes-Codex',
      risk: 'Destructive',
    });
  });

  it('builds browser settings guidance with a redacted token', () => {
    expect(
      buildSettingsViewModel(
        {
          daemonUrl: 'http://127.0.0.1:18787',
          apiToken: 'phase8-secret-token',
          operatorId: 'browser-operator',
        },
        false,
      ),
    ).toEqual({
      modeLabel: 'Browser preview',
      storageLabel: 'Browser localStorage',
      daemonUrl: 'http://127.0.0.1:18787',
      operatorId: 'browser-operator',
      tokenLabel: '****oken',
      tokenConfigured: true,
      canEditConnection: true,
    });
  });

  it('builds desktop settings guidance without revealing missing tokens', () => {
    expect(
      buildSettingsViewModel(
        {
          daemonUrl: 'http://127.0.0.1:18787',
          apiToken: '',
          operatorId: '',
        },
        true,
      ),
    ).toEqual({
      modeLabel: 'Tauri desktop',
      storageLabel: 'Environment variables',
      daemonUrl: 'http://127.0.0.1:18787',
      operatorId: 'local-gui',
      tokenLabel: 'not set',
      tokenConfigured: false,
      canEditConnection: false,
    });
  });

  it('filters log lines by case-insensitive search while preserving order', () => {
    const lines = [
      'INFO daemon ready',
      'WARN token redacted',
      'ERROR vLLM model unavailable',
      'INFO Hermes health ok',
    ];

    expect(buildFilteredLogLines(lines, 'vllm')).toEqual(['ERROR vLLM model unavailable']);
    expect(buildFilteredLogLines(lines, '   ')).toEqual(lines);
  });

  it('filters audit events by risk, requester, and query', () => {
    const events = [
      snapshot.audit[0],
      {
        id: 2,
        happened_at: '2026-05-06T10:02:00Z',
        requester_channel: 'gui',
        requester_user_id: 'desktop-operator',
        action: 'runtime.wsl.restart',
        risk_level: 'Destructive',
        summary: 'Restarted WSL distro',
      },
      {
        id: 3,
        happened_at: '2026-05-06T10:03:00Z',
        requester_channel: 'telegram',
        requester_user_id: 'admin',
        action: 'logs.tail',
        risk_level: 'ReadOnly',
        summary: 'Read daemon logs',
      },
    ];

    expect(buildAuditRiskOptions(events)).toEqual(['All', 'Destructive', 'NormalMutating', 'ReadOnly']);
    expect(
      buildFilteredAuditEvents(events, {
        riskLevel: 'Destructive',
        requester: 'gui',
        query: 'restart',
      }),
    ).toEqual([events[1]]);
  });

  it('translates common risk labels for localized UI while preserving unknown values', () => {
    const t = createTranslator('zh-CN');

    expect(translateAuditRiskFilter('All', t)).toBe('全部');
    expect(translateRiskLevel('ReadOnly', t)).toBe('只读');
    expect(translateRiskLevel('NormalMutating', t)).toBe('普通');
    expect(translateRiskLevel('CustomRisk', t)).toBe('CustomRisk');
  });
});
