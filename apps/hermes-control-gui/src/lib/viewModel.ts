import type {
  DashboardViewModel,
  GuiDashboardSnapshot,
  LogTargetViewModel,
  RouteOptionViewModel,
} from './types';

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
  ];
}
