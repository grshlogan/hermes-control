import type {
  ConfirmationPromptViewModel,
  AuditEventSummary,
  DashboardViewModel,
  GuiConnectionSettings,
  GuiDashboardSnapshot,
  HermesActionId,
  LogTargetViewModel,
  ModelActionId,
  ModelActionOptionViewModel,
  ModelActionProgressViewModel,
  OperationResponse,
  RouteOptionViewModel,
  RuntimeActionOptionViewModel,
  SettingsViewModel,
  StateSummary,
  WslActionId,
} from './types';
import type { createTranslator } from './i18n';

type Translator = ReturnType<typeof createTranslator>;

export const navigationSections = [
  { id: 'dashboard', label: 'Dashboard' },
  { id: 'route', label: 'AI Route' },
  { id: 'models', label: 'Local Models' },
  { id: 'runtime', label: 'Runtime' },
  { id: 'logs', label: 'Logs' },
  { id: 'audit', label: 'Audit' },
  { id: 'settings', label: 'Settings' },
] as const;

export const unsafeTauriPermissionPrefixes = ['shell:', 'fs:', 'process'] as const;

export function buildDashboardViewModel(snapshot: GuiDashboardSnapshot): DashboardViewModel {
  const readyModels = snapshot.models.filter((model) => model.ready).length;

  return {
    overallLabel: snapshot.status.overall.toUpperCase(),
    activeRoute: snapshot.active_route.active_profile_id ?? 'not set',
    lastKnownGoodRoute: snapshot.active_route.last_known_good_profile_id ?? 'not set',
    readyModels,
    totalModels: snapshot.models.length,
    wslState: snapshot.status.wsl?.state ?? 'Unknown',
    hermesReachable: snapshot.status.hermes.reachable,
  };
}

export function buildRouteOptions(snapshot: GuiDashboardSnapshot): RouteOptionViewModel[] {
  return snapshot.providers.map((provider) => ({
    id: provider.id,
    label: provider.display_name,
    kind: provider.kind,
    isActive: provider.id === snapshot.active_route.active_profile_id,
    isLastKnownGood: provider.id === snapshot.active_route.last_known_good_profile_id,
  }));
}

export function buildLogTargets(): LogTargetViewModel[] {
  return [
    { id: 'daemon', label: 'Daemon' },
    { id: 'bot', label: 'Bot' },
    { id: 'hermes', label: 'Hermes' },
    { id: 'vllm', label: 'vLLM' },
  ];
}

export function buildStateStoreSummary(
  state: StateSummary | null | undefined,
  t?: Translator,
): string {
  if (!state) {
    return t?.('status.unknown') ?? 'Unknown';
  }

  const present = t?.('status.present') ?? 'Present';
  const missing = t?.('status.missing') ?? 'Missing';
  const colon = t?.('punct.colon') ?? ': ';
  const stateDb = state.state_db_exists ? present : missing;
  const auditDb = state.audit_db_exists ? present : missing;

  return `${t?.('dashboard.stateDbShort') ?? 'State DB'}${colon}${stateDb} / ${t?.('dashboard.auditDbShort') ?? 'Audit DB'}${colon}${auditDb}`;
}

export function buildModelActionOptions(t?: Translator): ModelActionOptionViewModel[] {
  return [
    { id: 'Install', label: t?.('action.install') ?? 'Install', riskHint: t?.('risk.normal') ?? 'Normal' },
    { id: 'Start', label: t?.('action.start') ?? 'Start', riskHint: t?.('risk.normal') ?? 'Normal' },
    { id: 'Stop', label: t?.('action.stop') ?? 'Stop', riskHint: t?.('risk.destructive') ?? 'Destructive' },
    { id: 'Restart', label: t?.('action.restart') ?? 'Restart', riskHint: t?.('risk.destructive') ?? 'Destructive' },
    { id: 'Health', label: t?.('action.health') ?? 'Health', riskHint: t?.('risk.readOnly') ?? 'Read-only' },
    { id: 'Logs', label: t?.('action.logs') ?? 'Logs', riskHint: t?.('risk.readOnly') ?? 'Read-only' },
    { id: 'Benchmark', label: t?.('action.benchmark') ?? 'Benchmark', riskHint: t?.('risk.experimental') ?? 'Experimental' },
  ];
}

export function buildModelActionProgressMessage(
  modelId: string,
  action: ModelActionId,
  t?: Translator,
): ModelActionProgressViewModel {
  const label = buildModelActionOptions(t).find((option) => option.id === action)?.label ?? action;
  const colon = t?.('punct.colon') ?? ': ';
  const period = t?.('punct.period') ?? '. ';

  if (action === 'Start') {
    return {
      message: `${t?.('models.startingModel') ?? 'Starting model'}${colon}${modelId}${period}${t?.('models.longRunningStartupHint') ?? 'vLLM/MTP model loading can take several minutes. Check Logs for output.'}`,
      longRunning: true,
    };
  }

  if (action === 'Restart') {
    return {
      message: `${t?.('models.restartingModel') ?? 'Restarting model'}${colon}${modelId}${period}${t?.('models.longRunningStartupHint') ?? 'vLLM/MTP model loading can take several minutes. Check Logs for output.'}`,
      longRunning: true,
    };
  }

  return {
    message: `${t?.('models.submittingAction') ?? 'Submitting model action'}${colon}${modelId} / ${label}`,
    longRunning: false,
  };
}

export function buildWslActionOptions(t?: Translator): RuntimeActionOptionViewModel<WslActionId>[] {
  return [
    { id: 'Wake', label: t?.('action.wake') ?? 'Wake', riskHint: t?.('risk.normal') ?? 'Normal' },
    { id: 'StopDistro', label: t?.('action.stopDistro') ?? 'Stop', riskHint: t?.('risk.destructive') ?? 'Destructive' },
    { id: 'RestartDistro', label: t?.('action.restartDistro') ?? 'Restart', riskHint: t?.('risk.destructive') ?? 'Destructive' },
    { id: 'ShutdownAll', label: t?.('action.shutdownAll') ?? 'Shutdown all', riskHint: t?.('risk.destructive') ?? 'Destructive' },
  ];
}

export function buildHermesActionOptions(t?: Translator): RuntimeActionOptionViewModel<HermesActionId>[] {
  return [
    { id: 'Wake', label: t?.('action.wake') ?? 'Wake', riskHint: t?.('risk.normal') ?? 'Normal' },
    { id: 'Stop', label: t?.('action.stop') ?? 'Stop', riskHint: t?.('risk.destructive') ?? 'Destructive' },
    { id: 'Restart', label: t?.('action.restart') ?? 'Restart', riskHint: t?.('risk.destructive') ?? 'Destructive' },
    { id: 'Kill', label: t?.('action.kill') ?? 'Kill', riskHint: t?.('risk.destructive') ?? 'Destructive' },
  ];
}

export function buildConfirmationPrompt(
  response: OperationResponse | null,
): ConfirmationPromptViewModel | null {
  if (
    !response ||
    response.status !== 'confirmation_required' ||
    !response.confirmation_id ||
    !response.code_hint ||
    !response.expires_at
  ) {
    return null;
  }

  return {
    confirmationId: response.confirmation_id,
    codeHint: response.code_hint,
    expiresAt: response.expires_at,
    summary: response.summary,
    risk: response.risk,
  };
}

export function buildSettingsViewModel(
  settings: GuiConnectionSettings,
  isTauri: boolean,
  t?: Translator,
): SettingsViewModel {
  const token = settings.apiToken.trim();
  const operatorId = settings.operatorId.trim() || (isTauri ? 'local-gui' : 'browser-gui');

  return {
    modeLabel: isTauri ? t?.('settings.modeDesktop') ?? 'Tauri desktop' : t?.('settings.modeBrowser') ?? 'Browser preview',
    storageLabel: isTauri ? t?.('settings.storageEnv') ?? 'Environment variables' : t?.('settings.storageBrowser') ?? 'Browser localStorage',
    daemonUrl: settings.daemonUrl.trim() || 'http://127.0.0.1:18787',
    operatorId,
    tokenLabel: token ? redactToken(token) : t?.('settings.tokenNotSet') ?? 'not set',
    tokenConfigured: Boolean(token),
    canEditConnection: !isTauri,
  };
}

function redactToken(token: string): string {
  const suffix = token.slice(-4);
  return `****${suffix}`;
}

export function buildFilteredLogLines(lines: string[], query: string): string[] {
  const needle = query.trim().toLowerCase();
  if (!needle) {
    return lines;
  }

  return lines.filter((line) => line.toLowerCase().includes(needle));
}

export function buildAuditRiskOptions(events: AuditEventSummary[]): string[] {
  const risks = new Set(events.map((event) => event.risk_level).filter(Boolean));
  return ['All', ...Array.from(risks).sort()];
}

export function translateAuditRiskFilter(value: string, t: Translator): string {
  return value === 'All' ? t('risk.all') : translateRiskLevel(value, t);
}

export function translateRiskLevel(value: string, t: Translator): string {
  const normalized = value.toLowerCase();
  if (normalized === 'readonly' || normalized === 'read-only') {
    return t('risk.readOnly');
  }
  if (normalized === 'normalmutating' || normalized === 'normal') {
    return t('risk.normal');
  }
  if (normalized === 'destructive') {
    return t('risk.destructive');
  }
  if (normalized === 'experimental') {
    return t('risk.experimental');
  }

  return value;
}

export function buildFilteredAuditEvents(
  events: AuditEventSummary[],
  filters: { riskLevel: string; requester: string; query: string },
): AuditEventSummary[] {
  const riskLevel = filters.riskLevel.trim();
  const requester = filters.requester.trim().toLowerCase();
  const query = filters.query.trim().toLowerCase();

  return events.filter((event) => {
    const matchesRisk = !riskLevel || riskLevel === 'All' || event.risk_level === riskLevel;
    const matchesRequester =
      !requester ||
      event.requester_channel.toLowerCase().includes(requester) ||
      event.requester_user_id.toLowerCase().includes(requester);
    const searchable = [
      event.action,
      event.summary,
      event.risk_level,
      event.requester_channel,
      event.requester_user_id,
    ]
      .join(' ')
      .toLowerCase();
    const matchesQuery = !query || searchable.includes(query);

    return matchesRisk && matchesRequester && matchesQuery;
  });
}
