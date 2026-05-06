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
  endpoint: EndpointStatus;
  ready: boolean;
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
  model_runtime?: string | null;
  served_model_name?: string | null;
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
  isActive: boolean;
  isLastKnownGood: boolean;
}

export interface LogTargetViewModel {
  id: 'daemon' | 'bot' | 'hermes';
  label: string;
}

export interface OperationResponse {
  status: string;
  risk: string;
  summary: string;
  dry_run: boolean;
  commands?: Array<{ program: string; args: string[] }>;
  output?: string | null;
  confirmation_id?: string | null;
  code_hint?: string | null;
  expires_at?: string | null;
}

export interface GuiLogTail {
  target: string;
  path?: string | null;
  tail: number;
  lines: string[];
  detail?: string | null;
}
