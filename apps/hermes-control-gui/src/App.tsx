import type { ComponentType } from 'react';
import { useEffect, useMemo, useState } from 'react';
import {
  Activity,
  Bot,
  Database,
  FileClock,
  Gauge,
  History,
  Logs,
  MonitorCog,
  RefreshCw,
  Route,
  Server,
  Settings,
  ShieldCheck,
  TerminalSquare,
} from 'lucide-react';
import {
  loadDashboardSnapshot,
  loadLogTail,
  previewRouteRollback,
  previewRouteSwitch,
} from './lib/daemonClient';
import type { GuiDashboardSnapshot, GuiLogTail, OperationResponse } from './lib/types';
import {
  buildDashboardViewModel,
  buildLogTargets,
  buildRouteOptions,
  navigationSections,
} from './lib/viewModel';
import './styles.css';

const navIcons = {
  dashboard: Gauge,
  route: Route,
  models: Server,
  runtime: MonitorCog,
  logs: Logs,
  audit: FileClock,
  settings: Settings,
} as const;

export default function App() {
  const [snapshot, setSnapshot] = useState<GuiDashboardSnapshot | null>(null);
  const [selectedSection, setSelectedSection] = useState('dashboard');
  const [statusMessage, setStatusMessage] = useState('Connecting to daemon');
  const [loading, setLoading] = useState(false);
  const [selectedProfileId, setSelectedProfileId] = useState('');
  const [routePreview, setRoutePreview] = useState<OperationResponse | null>(null);
  const [routeMessage, setRouteMessage] = useState('Select a provider and preview before switching.');
  const [selectedLogTarget, setSelectedLogTarget] = useState<'daemon' | 'bot' | 'hermes'>('daemon');
  const [logTail, setLogTail] = useState<GuiLogTail | null>(null);
  const [logMessage, setLogMessage] = useState('Choose a daemon-owned log target.');

  async function refresh() {
    setLoading(true);
    try {
      const next = await loadDashboardSnapshot();
      setSnapshot(next);
      setSelectedProfileId((current) => current || next.active_route.active_profile_id || '');
      setStatusMessage('Daemon snapshot loaded');
    } catch (error) {
      setStatusMessage(error instanceof Error ? error.message : 'Daemon snapshot unavailable');
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  const model = useMemo(
    () => (snapshot ? buildDashboardViewModel(snapshot) : null),
    [snapshot],
  );
  const routeOptions = useMemo(() => (snapshot ? buildRouteOptions(snapshot) : []), [snapshot]);
  const logTargets = useMemo(() => buildLogTargets(), []);

  async function previewSelectedRoute() {
    if (!selectedProfileId) {
      setRouteMessage('Select a route profile first.');
      return;
    }

    setRouteMessage('Requesting daemon dry-run preview');
    try {
      const preview = await previewRouteSwitch(selectedProfileId);
      setRoutePreview(preview);
      setRouteMessage('Route switch preview loaded');
    } catch (error) {
      setRouteMessage(error instanceof Error ? error.message : 'Route preview failed');
    }
  }

  async function previewRollback() {
    setRouteMessage('Requesting rollback dry-run preview');
    try {
      const preview = await previewRouteRollback();
      setRoutePreview(preview);
      setRouteMessage('Rollback preview loaded');
    } catch (error) {
      setRouteMessage(error instanceof Error ? error.message : 'Rollback preview failed');
    }
  }

  async function refreshLogs() {
    setLogMessage(`Loading ${selectedLogTarget} logs`);
    try {
      const next = await loadLogTail(selectedLogTarget);
      setLogTail(next);
      setLogMessage(next.detail ?? `${next.lines.length} line(s) loaded`);
    } catch (error) {
      setLogMessage(error instanceof Error ? error.message : 'Log tail failed');
    }
  }

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <ShieldCheck size={24} />
          <div>
            <strong>Hermes Control</strong>
            <span>Local stack console</span>
          </div>
        </div>

        <nav className="nav-list" aria-label="Main sections">
          {navigationSections.map((section) => {
            const Icon = navIcons[section.id];
            return (
              <button
                key={section.id}
                className={section.id === selectedSection ? 'nav-item active' : 'nav-item'}
                onClick={() => setSelectedSection(section.id)}
                type="button"
                title={section.label}
              >
                <Icon size={18} />
                <span>{section.label}</span>
              </button>
            );
          })}
        </nav>
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">Phase8 GUI</p>
            <h1>Operations dashboard</h1>
          </div>
          <button className="icon-button" onClick={refresh} type="button" title="Refresh daemon snapshot">
            <RefreshCw size={18} className={loading ? 'spin' : undefined} />
          </button>
        </header>

        <section className="status-strip" aria-label="Runtime status">
          <Metric icon={Activity} label="Overall" value={model?.overallLabel ?? 'OFFLINE'} />
          <Metric icon={Route} label="Active route" value={model?.activeRoute ?? 'not loaded'} />
          <Metric icon={Bot} label="Models ready" value={model ? `${model.readyModels}/${model.totalModels}` : '0/0'} />
          <Metric icon={TerminalSquare} label="WSL" value={model?.wslState ?? 'Unknown'} />
          <Metric icon={Database} label="State DB" value={snapshot?.status.state.state_db_exists ? 'Present' : 'Unknown'} />
        </section>

        <section className="main-grid">
          <section className="primary-pane" aria-label="Selected control surface">
            {selectedSection === 'dashboard' && <Dashboard snapshot={snapshot} statusMessage={statusMessage} />}
            {selectedSection === 'route' && (
              <RoutePane
                options={routeOptions}
                selectedProfileId={selectedProfileId}
                setSelectedProfileId={setSelectedProfileId}
                previewSelectedRoute={previewSelectedRoute}
                previewRollback={previewRollback}
                routePreview={routePreview}
                routeMessage={routeMessage}
              />
            )}
            {selectedSection === 'models' && <ModelsPane snapshot={snapshot} />}
            {selectedSection === 'runtime' && <RuntimePane snapshot={snapshot} />}
            {selectedSection === 'logs' && (
              <LogsPane
                targets={logTargets}
                selectedTarget={selectedLogTarget}
                setSelectedTarget={setSelectedLogTarget}
                refreshLogs={refreshLogs}
                logTail={logTail}
                logMessage={logMessage}
              />
            )}
            {selectedSection === 'audit' && <AuditPane snapshot={snapshot} />}
            {selectedSection === 'settings' && <SettingsPane />}
          </section>

          <aside className="inspector" aria-label="Safety boundary">
            <p className="eyebrow">Boundary</p>
            <h2>Daemon client only</h2>
            <p>
              GUI actions use typed daemon APIs. No shell, filesystem, or raw process authority is exposed
              through Tauri capabilities.
            </p>
            <div className="boundary-list">
              <span>core:default</span>
              <span>Authorization: Bearer</span>
              <span>Requester: gui</span>
            </div>
          </aside>
        </section>
      </section>
    </main>
  );
}

function Metric({
  icon: Icon,
  label,
  value,
}: {
  icon: ComponentType<{ size?: number }>;
  label: string;
  value: string;
}) {
  return (
    <div className="metric">
      <Icon size={18} />
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function Dashboard({ snapshot, statusMessage }: { snapshot: GuiDashboardSnapshot | null; statusMessage: string }) {
  return (
    <>
      <PanelHeader title="Dashboard" note={statusMessage} />
      <div className="detail-table">
        <Row label="Hermes health" value={snapshot?.status.hermes.message ?? 'Waiting for daemon'} />
        <Row label="Hermes URL" value={snapshot?.status.hermes.url ?? 'http://127.0.0.1:8642/health'} />
        <Row label="Last-known-good route" value={snapshot?.active_route.last_known_good_profile_id ?? 'not loaded'} />
        <Row label="Audit DB" value={snapshot?.status.state.audit_db_exists ? 'Present' : 'Unknown'} />
      </div>
    </>
  );
}

function RoutePane({
  options,
  selectedProfileId,
  setSelectedProfileId,
  previewSelectedRoute,
  previewRollback,
  routePreview,
  routeMessage,
}: {
  options: ReturnType<typeof buildRouteOptions>;
  selectedProfileId: string;
  setSelectedProfileId: (value: string) => void;
  previewSelectedRoute: () => void;
  previewRollback: () => void;
  routePreview: OperationResponse | null;
  routeMessage: string;
}) {
  return (
    <>
      <PanelHeader title="AI Route" note="Switch and rollback controls will use daemon confirmation flow." />
      <div className="action-row">
        <label>
          <span>Route profile</span>
          <select value={selectedProfileId} onChange={(event) => setSelectedProfileId(event.target.value)}>
            <option value="">Select profile</option>
            {options.map((option) => (
              <option key={option.id} value={option.id}>
                {option.label} / {option.kind}
              </option>
            ))}
          </select>
        </label>
        <button type="button" onClick={previewSelectedRoute}>
          Preview switch
        </button>
        <button type="button" onClick={previewRollback}>
          Preview rollback
        </button>
      </div>
      <p className="inline-status">{routeMessage}</p>
      <div className="detail-table">
        {options.map((option) => (
          <Row
            key={option.id}
            label={option.id}
            value={[
              option.label,
              option.kind,
              option.isActive ? 'active' : '',
              option.isLastKnownGood ? 'last-known-good' : '',
            ]
              .filter(Boolean)
              .join(' / ')}
          />
        ))}
      </div>
      {routePreview && <OperationPreview response={routePreview} />}
    </>
  );
}

function ModelsPane({ snapshot }: { snapshot: GuiDashboardSnapshot | null }) {
  return (
    <>
      <PanelHeader title="Local Models" note="Readiness mirrors daemon model status." />
      <div className="model-list">
        {(snapshot?.models ?? []).map((model) => (
          <div className="model-row" key={model.variant_id}>
            <span className={model.ready ? 'dot ok' : 'dot down'} />
            <div>
              <strong>{model.variant_id}</strong>
              <span>{model.served_model_name}</span>
            </div>
            <code>{model.endpoint.url}</code>
          </div>
        ))}
      </div>
    </>
  );
}

function RuntimePane({ snapshot }: { snapshot: GuiDashboardSnapshot | null }) {
  return (
    <>
      <PanelHeader title="WSL and Hermes Runtime" note="Destructive operations require daemon confirmation." />
      <div className="detail-table">
        <Row label="WSL distro" value={snapshot?.status.wsl?.name ?? 'not loaded'} />
        <Row label="WSL state" value={snapshot?.status.wsl?.state ?? 'Unknown'} />
        <Row label="Hermes reachable" value={snapshot?.status.hermes.reachable ? 'Yes' : 'Unknown'} />
      </div>
    </>
  );
}

function LogsPane({
  targets,
  selectedTarget,
  setSelectedTarget,
  refreshLogs,
  logTail,
  logMessage,
}: {
  targets: ReturnType<typeof buildLogTargets>;
  selectedTarget: 'daemon' | 'bot' | 'hermes';
  setSelectedTarget: (value: 'daemon' | 'bot' | 'hermes') => void;
  refreshLogs: () => void;
  logTail: GuiLogTail | null;
  logMessage: string;
}) {
  return (
    <>
      <PanelHeader title="Logs" note="Daemon-owned log targets only." />
      <div className="action-row">
        <label>
          <span>Target</span>
          <select
            value={selectedTarget}
            onChange={(event) => setSelectedTarget(event.target.value as 'daemon' | 'bot' | 'hermes')}
          >
            {targets.map((target) => (
              <option key={target.id} value={target.id}>
                {target.label}
              </option>
            ))}
          </select>
        </label>
        <button type="button" onClick={refreshLogs}>
          Tail logs
        </button>
      </div>
      <p className="inline-status">{logMessage}</p>
      <pre className="log-preview">
        {(logTail?.lines.length ? logTail.lines : ['No log lines loaded yet.']).join('\n')}
      </pre>
    </>
  );
}

function AuditPane({ snapshot }: { snapshot: GuiDashboardSnapshot | null }) {
  return (
    <>
      <PanelHeader title="Audit" note="Recent daemon audit events." />
      <div className="audit-list">
        {(snapshot?.audit ?? []).map((event) => (
          <div className="audit-row" key={event.id}>
            <History size={16} />
            <span>{event.action}</span>
            <strong>{event.risk_level}</strong>
          </div>
        ))}
      </div>
    </>
  );
}

function SettingsPane() {
  return (
    <>
      <PanelHeader title="Settings" note="Environment-driven daemon URL and token remain outside committed files." />
      <div className="detail-table">
        <Row label="Daemon URL" value="HERMES_CONTROL_DAEMON_URL" />
        <Row label="API token" value="HERMES_CONTROL_API_TOKEN" />
        <Row label="Operator" value="HERMES_CONTROL_GUI_OPERATOR_ID" />
      </div>
    </>
  );
}

function PanelHeader({ title, note }: { title: string; note: string }) {
  return (
    <header className="panel-header">
      <h2>{title}</h2>
      <p>{note}</p>
    </header>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="detail-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function OperationPreview({ response }: { response: OperationResponse }) {
  return (
    <section className="operation-preview" aria-label="Daemon dry-run preview">
      <PanelHeader title="Dry-run preview" note={response.summary} />
      <div className="detail-table">
        <Row label="Status" value={response.status} />
        <Row label="Risk" value={response.risk} />
        <Row label="Dry run" value={String(response.dry_run)} />
      </div>
      {!!response.commands?.length && (
        <pre className="command-preview">
          {response.commands.map((command) => `${command.program} ${command.args.join(' ')}`).join('\n')}
        </pre>
      )}
    </section>
  );
}
