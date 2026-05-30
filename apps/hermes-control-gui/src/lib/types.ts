export type HealthStatus = 'Ok' | 'Degraded' | 'Down';

export interface EndpointStatus {
  url: string;
  reachable: boolean;
  status_code?: number;
  message: string;
}

export interface WslDistroStatus {
  name: string;
  state: string;
  version?: number;
  default: boolean;
}

export interface ModelRuntimeSummary {
  runtime_id: string;
  variant_id: string;
  served_model_name: string;
  model_root?: string | null;
  endpoint: EndpointStatus;
  ready: boolean;
}

export type ModelActionId =
  | 'Install'
  | 'Start'
  | 'Stop'
  | 'Restart'
  | 'Health'
  | 'Logs'
  | 'Benchmark';

export interface ModelActionOptionViewModel {
  id: ModelActionId;
  label: string;
  riskHint: string;
}

export interface ModelActionProgressViewModel {
  message: string;
  longRunning: boolean;
}

export type WslActionId = 'Wake' | 'StopDistro' | 'RestartDistro' | 'ShutdownAll';
export type HermesActionId = 'Wake' | 'Stop' | 'Restart' | 'Kill';
export type OpenWebUiActionId = 'Wake' | 'Stop' | 'Restart' | 'Status';

export interface RuntimeActionOptionViewModel<T extends string> {
  id: T;
  label: string;
  riskHint: string;
}

export interface StateSummary {
  state_db_exists: boolean;
  audit_db_exists: boolean;
}

export interface ReadOnlyStatus {
  wsl?: WslDistroStatus | null;
  hermes: EndpointStatus;
  models: ModelRuntimeSummary[];
  state: StateSummary;
  overall: HealthStatus;
}

export interface ActiveRouteStatus {
  active_profile_id?: string | null;
  last_known_good_profile_id?: string | null;
}

export interface ProviderConfig {
  id: string;
  kind: string;
  display_name: string;
  base_url?: string | null;
  api_key_ref?: string | null;
  models: string[];
  default_account_id?: string | null;
  default_model?: string | null;
  anthropic_defaults?: {
    model?: string | null;
    sonnet?: string | null;
    haiku?: string | null;
    opus?: string | null;
  } | null;
  runtime_env?: Record<string, string>;
  accounts?: ProviderAccountConfig[];
  model_runtime?: string | null;
  served_model_name?: string | null;
}

export interface ProviderAccountConfig {
  id: string;
  display_name: string;
  secret_ref: string;
  secret_env_key: string;
  secret_source?: string;
  enabled: boolean;
  priority: number;
}

export interface AuditEventSummary {
  id: number;
  happened_at: string;
  requester_channel: string;
  requester_user_id: string;
  action: string;
  risk_level: string;
  summary: string;
}

export interface GuiDashboardSnapshot {
  status: ReadOnlyStatus;
  active_route: ActiveRouteStatus;
  providers: ProviderConfig[];
  models: ModelRuntimeSummary[];
  audit: AuditEventSummary[];
}

export interface ProviderImportPreviewResponse {
  status: string;
  source: string;
  dry_run: boolean;
  summary: string;
  provider_count: number;
  providers: ProviderConfig[];
}

export interface ProviderImportPreviewRowViewModel {
  id: string;
  label: string;
  summary: string;
}

export interface DashboardViewModel {
  overallLabel: string;
  activeRoute: string;
  lastKnownGoodRoute: string;
  readyModels: number;
  totalModels: number;
  wslState: string;
  hermesReachable: boolean;
}

export interface RouteOptionViewModel {
  id: string;
  label: string;
  kind: string;
  baseUrl: string;
  defaultModel: string;
  accountSummary: string;
  secretEnvKey: string;
  runtimeEnvKeys: string[];
  isActive: boolean;
  isLastKnownGood: boolean;
}

export interface LogTargetViewModel {
  id: LogTargetId;
  label: string;
}

export type LogTargetId = 'daemon' | 'bot' | 'hermes' | 'vllm';

export interface GuiConnectionSettings {
  daemonUrl: string;
  apiToken: string;
  operatorId: string;
}

export interface GuiConnectionSummary {
  daemon_url: string;
  operator_id: string;
  token_configured: boolean;
  token_label: string;
}

export interface SettingsViewModel {
  modeLabel: string;
  storageLabel: string;
  daemonUrl: string;
  operatorId: string;
  tokenLabel: string;
  tokenConfigured: boolean;
  canEditConnection: boolean;
}

export interface OperationResponse {
  status: string;
  risk: string;
  summary: string;
  dry_run: boolean;
  commands?: Array<{ program: string; args: string[]; env?: Record<string, string> }>;
  output?: string | null;
  confirmation_id?: string | null;
  code_hint?: string | null;
  expires_at?: string | null;
}

export interface ConfirmationLifecycleResponse {
  status: string;
  confirmation_id: string;
  summary: string;
  execution_status?: string | null;
}

export interface ConfirmationPromptViewModel {
  confirmationId: string;
  codeHint: string;
  expiresAt: string;
  summary: string;
  risk: string;
}

export interface GuiLogTail {
  target: string;
  path?: string | null;
  tail: number;
  lines: string[];
  detail?: string | null;
}
