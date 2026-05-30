import type { ComponentType } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';
import {
  Activity,
  Bot,
  Check,
  FileClock,
  Gauge,
  History,
  Info,
  Logs,
  MonitorCog,
  Play,
  RefreshCw,
  Route,
  RotateCcw,
  Search,
  Server,
  Settings,
  ShieldCheck,
  TerminalSquare,
  X,
} from 'lucide-react';
import {
  cancelOperation,
  confirmOperation,
  executeRouteRollback,
  executeRouteSwitch,
  loadDashboardSnapshot,
  loadConnectionSummary,
  loadLogTail,
  previewRouteRollback,
  previewRouteSwitch,
  executeModelAction,
  executeHermesAction,
  executeOpenWebUiAction,
  executeWslAction,
  previewModelAction,
  previewHermesAction,
  previewOpenWebUiAction,
  previewWslAction,
  isDesktopRuntime,
  readBrowserSettings,
  saveBrowserSettings,
} from './lib/daemonClient';
import type {
  GuiConnectionSettings,
  GuiDashboardSnapshot,
  GuiLogTail,
  HermesActionId,
  LogTargetId,
  ModelActionId,
  OpenWebUiActionId,
  OperationResponse,
  WslActionId,
} from './lib/types';
import {
  buildConfirmationPrompt,
  buildAuditRiskOptions,
  buildDashboardViewModel,
  buildFilteredAuditEvents,
  buildFilteredLogLines,
  buildHermesActionOptions,
  buildLogTargets,
  buildModelActionProgressMessage,
  buildModelActionOptions,
  buildOpenWebUiActionOptions,
  buildRouteOptions,
  buildSettingsViewModel,
  buildStateStoreSummary,
  buildWslActionOptions,
  navigationSections,
  translateAuditRiskFilter,
  translateRiskLevel,
} from './lib/viewModel';
import {
  DEFAULT_LANGUAGE,
  type LanguageId,
  createTranslator,
  languageOptions,
  normalizeLanguage,
} from './lib/i18n';
import './styles.css';

const LANGUAGE_STORAGE_KEY = 'hermes-control-gui-language';

const navIcons = {
  dashboard: Gauge,
  route: Route,
  models: Server,
  runtime: MonitorCog,
  logs: Logs,
  audit: FileClock,
  info: Info,
  settings: Settings,
} as const;

const navLabelKeys = {
  dashboard: 'nav.dashboard',
  route: 'nav.route',
  models: 'nav.models',
  runtime: 'nav.runtime',
  logs: 'nav.logs',
  audit: 'nav.audit',
  info: 'nav.info',
  settings: 'nav.settings',
} as const;

export default function App() {
  const [snapshot, setSnapshot] = useState<GuiDashboardSnapshot | null>(null);
  const [selectedSection, setSelectedSection] = useState('dashboard');
  const [language, setLanguageState] = useState<LanguageId>(() => readLanguageSetting());
  const t = createTranslator(language);
  const [statusMessage, setStatusMessage] = useState(() => t('status.connecting'));
  const [loading, setLoading] = useState(false);
  const [selectedProfileId, setSelectedProfileId] = useState('');
  const [routePreview, setRoutePreview] = useState<OperationResponse | null>(null);
  const [routeMessage, setRouteMessage] = useState(() => t('route.initial'));
  const [routeBusy, setRouteBusy] = useState(false);
  const [confirmationCode, setConfirmationCode] = useState('');
  const [selectedModelId, setSelectedModelId] = useState('');
  const [selectedModelAction, setSelectedModelAction] = useState<ModelActionId>('Health');
  const [modelResponse, setModelResponse] = useState<OperationResponse | null>(null);
  const [modelMessage, setModelMessage] = useState(() => t('models.initial'));
  const [modelBusy, setModelBusy] = useState(false);
  const [modelConfirmationCode, setModelConfirmationCode] = useState('');
  const [selectedWslAction, setSelectedWslAction] = useState<WslActionId>('Wake');
  const [selectedHermesAction, setSelectedHermesAction] = useState<HermesActionId>('Wake');
  const [selectedOpenWebUiAction, setSelectedOpenWebUiAction] = useState<OpenWebUiActionId>('Status');
  const [runtimeResponse, setRuntimeResponse] = useState<OperationResponse | null>(null);
  const [runtimeMessage, setRuntimeMessage] = useState(() => t('runtime.initial'));
  const [runtimeBusy, setRuntimeBusy] = useState(false);
  const [runtimeConfirmationCode, setRuntimeConfirmationCode] = useState('');
  const [selectedLogTarget, setSelectedLogTarget] = useState<LogTargetId>('daemon');
  const [logTailSize, setLogTailSize] = useState(200);
  const [logFilter, setLogFilter] = useState('');
  const [logTail, setLogTail] = useState<GuiLogTail | null>(null);
  const [logMessage, setLogMessage] = useState(() => t('logs.initial'));
  const [auditRiskFilter, setAuditRiskFilter] = useState('All');
  const [auditRequesterFilter, setAuditRequesterFilter] = useState('');
  const [auditQueryFilter, setAuditQueryFilter] = useState('');
  const [connectionSettings, setConnectionSettings] = useState<GuiConnectionSettings>(() =>
    readBrowserSettings(),
  );
  const [settingsMessage, setSettingsMessage] = useState(() =>
    createTranslator(readLanguageSetting())('settings.localMessage'),
  );
  const [settingsBusy, setSettingsBusy] = useState(false);
  const initialLoadStarted = useRef(false);
  const desktopRuntime = isDesktopRuntime();

  async function refresh() {
    setLoading(true);
    try {
      const next = await loadDashboardSnapshot();
      setSnapshot(next);
      setSelectedProfileId((current) => current || next.active_route.active_profile_id || '');
      setSelectedModelId((current) => current || next.models[0]?.variant_id || '');
      setStatusMessage(t('status.snapshotLoaded'));
    } catch (error) {
      setStatusMessage(error instanceof Error ? error.message : t('status.snapshotUnavailable'));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (initialLoadStarted.current) {
      return;
    }
    initialLoadStarted.current = true;
    void refresh();
    void refreshConnectionSettings();
  }, []);

  const model = useMemo(
    () => (snapshot ? buildDashboardViewModel(snapshot) : null),
    [snapshot],
  );
  const routeOptions = useMemo(() => (snapshot ? buildRouteOptions(snapshot) : []), [snapshot]);
  const logTargets = useMemo(() => buildLogTargets(), []);
  const modelActionOptions = useMemo(() => buildModelActionOptions(t), [language]);
  const wslActionOptions = useMemo(() => buildWslActionOptions(t), [language]);
  const hermesActionOptions = useMemo(() => buildHermesActionOptions(t), [language]);
  const openWebUiActionOptions = useMemo(() => buildOpenWebUiActionOptions(t), [language]);
  const confirmationPrompt = useMemo(
    () => buildConfirmationPrompt(routePreview),
    [routePreview],
  );
  const modelConfirmationPrompt = useMemo(
    () => buildConfirmationPrompt(modelResponse),
    [modelResponse],
  );
  const runtimeConfirmationPrompt = useMemo(
    () => buildConfirmationPrompt(runtimeResponse),
    [runtimeResponse],
  );

  async function previewSelectedRoute() {
    if (!selectedProfileId) {
      setRouteMessage(t('route.selectFirst'));
      return;
    }

    setRouteMessage(t('route.requestingPreview'));
    try {
      const preview = await previewRouteSwitch(selectedProfileId);
      setRoutePreview(preview);
      setRouteMessage(t('route.previewLoaded'));
    } catch (error) {
      setRouteMessage(error instanceof Error ? error.message : t('route.previewFailed'));
    }
  }

  async function previewRollback() {
    setRouteMessage(t('route.requestingRollbackPreview'));
    try {
      const preview = await previewRouteRollback();
      setRoutePreview(preview);
      setRouteMessage(t('route.rollbackPreviewLoaded'));
    } catch (error) {
      setRouteMessage(error instanceof Error ? error.message : t('route.rollbackPreviewFailed'));
    }
  }

  async function executeSelectedRoute() {
    if (!selectedProfileId) {
      setRouteMessage(t('route.selectFirst'));
      return;
    }

    setRouteBusy(true);
    setRouteMessage(t('route.submittingSwitch'));
    try {
      const response = await executeRouteSwitch(selectedProfileId);
      setRoutePreview(response);
      setConfirmationCode('');
      if (response.status === 'confirmation_required') {
        setRouteMessage(t('operation.confirmationRequired'));
        return;
      }
      setRouteMessage(response.summary || t('route.switchSubmitted'));
      await refresh();
    } catch (error) {
      setRouteMessage(formatRequestError(error, t('route.switchFailed'), t('route.conflict')));
    } finally {
      setRouteBusy(false);
    }
  }

  async function executeRollback() {
    setRouteBusy(true);
    setRouteMessage(t('route.submittingRollback'));
    try {
      const response = await executeRouteRollback();
      setRoutePreview(response);
      setConfirmationCode('');
      if (response.status === 'confirmation_required') {
        setRouteMessage(t('operation.confirmationRequired'));
        return;
      }
      setRouteMessage(response.summary || t('route.rollbackSubmitted'));
      await refresh();
    } catch (error) {
      setRouteMessage(formatRequestError(error, t('route.rollbackFailed'), t('route.conflict')));
    } finally {
      setRouteBusy(false);
    }
  }

  async function confirmPendingOperation() {
    if (!confirmationPrompt) {
      setRouteMessage(t('operation.noPendingConfirmation'));
      return;
    }
    if (!confirmationCode.trim()) {
      setRouteMessage(t('operation.enterCode'));
      return;
    }

    setRouteBusy(true);
    setRouteMessage(t('operation.confirming'));
    try {
      const response = await confirmOperation(confirmationCode.trim());
      setRoutePreview(null);
      setConfirmationCode('');
      setRouteMessage(`${response.status}: ${response.summary}`);
      await refresh();
    } catch (error) {
      setRouteMessage(error instanceof Error ? error.message : t('operation.confirmFailed'));
    } finally {
      setRouteBusy(false);
    }
  }

  async function cancelPendingOperation() {
    if (!confirmationPrompt) {
      setRouteMessage(t('operation.noPendingConfirmation'));
      return;
    }

    setRouteBusy(true);
    setRouteMessage(t('operation.cancelling'));
    try {
      const response = await cancelOperation();
      setRoutePreview(null);
      setConfirmationCode('');
      setRouteMessage(`${response.status}: ${response.summary}`);
      await refresh();
    } catch (error) {
      setRouteMessage(error instanceof Error ? error.message : t('operation.cancelFailed'));
    } finally {
      setRouteBusy(false);
    }
  }

  async function previewSelectedModelAction() {
    if (!selectedModelId) {
      setModelMessage(t('models.selectFirst'));
      return;
    }

    setModelBusy(true);
    setModelMessage(`${t('models.previewing')} ${selectedModelAction.toLowerCase()}`);
    try {
      const response = await previewModelAction(selectedModelId, selectedModelAction);
      setModelResponse(response);
      setModelMessage(response.summary || t('models.previewLoaded'));
    } catch (error) {
      setModelMessage(error instanceof Error ? error.message : t('models.previewFailed'));
    } finally {
      setModelBusy(false);
    }
  }

  async function executeSelectedModelAction() {
    if (!selectedModelId) {
      setModelMessage(t('models.selectFirst'));
      return;
    }

    setModelBusy(true);
    const progress = buildModelActionProgressMessage(selectedModelId, selectedModelAction, t);
    setModelMessage(progress.message);
    try {
      const response = await executeModelAction(selectedModelId, selectedModelAction);
      setModelResponse(response);
      setModelConfirmationCode('');
      if (response.status === 'confirmation_required') {
        setModelMessage(t('operation.confirmationRequired'));
        return;
      }
      setModelMessage(response.summary || t('models.submitted'));
      await refresh();
    } catch (error) {
      setModelMessage(formatRequestError(error, t('models.failed'), t('models.conflict')));
    } finally {
      setModelBusy(false);
    }
  }

  async function confirmPendingModelOperation() {
    if (!modelConfirmationPrompt) {
      setModelMessage(t('models.noPendingConfirmation'));
      return;
    }
    if (!modelConfirmationCode.trim()) {
      setModelMessage(t('operation.enterCode'));
      return;
    }

    setModelBusy(true);
    setModelMessage(t('models.confirming'));
    try {
      const response = await confirmOperation(modelConfirmationCode.trim());
      setModelResponse(null);
      setModelConfirmationCode('');
      setModelMessage(`${response.status}: ${response.summary}`);
      await refresh();
    } catch (error) {
      setModelMessage(error instanceof Error ? error.message : t('models.confirmFailed'));
    } finally {
      setModelBusy(false);
    }
  }

  async function cancelPendingModelOperation() {
    if (!modelConfirmationPrompt) {
      setModelMessage(t('models.noPendingConfirmation'));
      return;
    }

    setModelBusy(true);
    setModelMessage(t('models.cancelling'));
    try {
      const response = await cancelOperation();
      setModelResponse(null);
      setModelConfirmationCode('');
      setModelMessage(`${response.status}: ${response.summary}`);
      await refresh();
    } catch (error) {
      setModelMessage(error instanceof Error ? error.message : t('models.cancelFailed'));
    } finally {
      setModelBusy(false);
    }
  }

  async function previewSelectedRuntimeAction(target: 'wsl' | 'hermes' | 'openwebui') {
    setRuntimeBusy(true);
    setRuntimeMessage(`${t('runtime.previewing')}: ${target}`);
    try {
      const response = await previewRuntimeActionForTarget(
        target,
        selectedWslAction,
        selectedHermesAction,
        selectedOpenWebUiAction,
      );
      setRuntimeResponse(response);
      setRuntimeMessage(response.summary || t('runtime.previewLoaded'));
    } catch (error) {
      setRuntimeMessage(error instanceof Error ? error.message : t('runtime.previewFailed'));
    } finally {
      setRuntimeBusy(false);
    }
  }

  async function executeSelectedRuntimeAction(target: 'wsl' | 'hermes' | 'openwebui') {
    setRuntimeBusy(true);
    setRuntimeMessage(`${t('runtime.submitting')}: ${target}`);
    try {
      const response = await executeRuntimeActionForTarget(
        target,
        selectedWslAction,
        selectedHermesAction,
        selectedOpenWebUiAction,
      );
      setRuntimeResponse(response);
      setRuntimeConfirmationCode('');
      if (response.status === 'confirmation_required') {
        setRuntimeMessage(t('operation.confirmationRequired'));
        return;
      }
      setRuntimeMessage(response.summary || t('runtime.submitted'));
      await refresh();
    } catch (error) {
      setRuntimeMessage(formatRequestError(error, t('runtime.failed'), t('runtime.conflict')));
    } finally {
      setRuntimeBusy(false);
    }
  }

  async function confirmPendingRuntimeOperation() {
    if (!runtimeConfirmationPrompt) {
      setRuntimeMessage(t('runtime.noPendingConfirmation'));
      return;
    }
    if (!runtimeConfirmationCode.trim()) {
      setRuntimeMessage(t('operation.enterCode'));
      return;
    }

    setRuntimeBusy(true);
    setRuntimeMessage(t('runtime.confirming'));
    try {
      const response = await confirmOperation(runtimeConfirmationCode.trim());
      setRuntimeResponse(null);
      setRuntimeConfirmationCode('');
      setRuntimeMessage(`${response.status}: ${response.summary}`);
      await refresh();
    } catch (error) {
      setRuntimeMessage(error instanceof Error ? error.message : t('runtime.confirmFailed'));
    } finally {
      setRuntimeBusy(false);
    }
  }

  async function cancelPendingRuntimeOperation() {
    if (!runtimeConfirmationPrompt) {
      setRuntimeMessage(t('runtime.noPendingConfirmation'));
      return;
    }

    setRuntimeBusy(true);
    setRuntimeMessage(t('runtime.cancelling'));
    try {
      const response = await cancelOperation();
      setRuntimeResponse(null);
      setRuntimeConfirmationCode('');
      setRuntimeMessage(`${response.status}: ${response.summary}`);
      await refresh();
    } catch (error) {
      setRuntimeMessage(error instanceof Error ? error.message : t('runtime.cancelFailed'));
    } finally {
      setRuntimeBusy(false);
    }
  }

  async function refreshLogs() {
    setLogMessage(`${t('logs.loading')}: ${selectedLogTarget}`);
    try {
      const next = await loadLogTail(selectedLogTarget, logTailSize);
      setLogTail(next);
      setLogMessage(next.detail ?? `${next.lines.length} ${t('logs.loaded')}`);
    } catch (error) {
      setLogMessage(error instanceof Error ? error.message : t('logs.failed'));
    }
  }

  async function refreshConnectionSettings() {
    try {
      const next = await loadConnectionSummary();
      setConnectionSettings(next);
      setSettingsMessage(t('settings.loaded'));
    } catch (error) {
      setSettingsMessage(error instanceof Error ? error.message : t('settings.unavailable'));
    }
  }

  function setLanguage(next: LanguageId) {
    setLanguageState(next);
    if (typeof window !== 'undefined') {
      window.localStorage.setItem(LANGUAGE_STORAGE_KEY, next);
    }
    setSettingsMessage(createTranslator(next)('settings.languageSaved'));
  }

  async function saveConnectionSettings() {
    setSettingsBusy(true);
    try {
      if (desktopRuntime) {
        await refreshConnectionSettings();
        setSettingsMessage(t('settings.desktopEnv'));
        return;
      }

      saveBrowserSettings(connectionSettings);
      setSettingsMessage(t('settings.browserSaved'));
    } catch (error) {
      setSettingsMessage(error instanceof Error ? error.message : t('settings.saveFailed'));
    } finally {
      setSettingsBusy(false);
    }
  }

  async function testConnectionSettings() {
    setSettingsBusy(true);
    setSettingsMessage(t('settings.testing'));
    try {
      if (!desktopRuntime) {
        saveBrowserSettings(connectionSettings);
      }
      const next = await loadDashboardSnapshot();
      setSnapshot(next);
      setSelectedProfileId((current) => current || next.active_route.active_profile_id || '');
      setSelectedModelId((current) => current || next.models[0]?.variant_id || '');
      setStatusMessage(t('status.snapshotLoadedFromSettings'));
      setSettingsMessage(t('settings.connected'));
    } catch (error) {
      setSettingsMessage(error instanceof Error ? error.message : t('settings.connectionFailed'));
    } finally {
      setSettingsBusy(false);
    }
  }

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <ShieldCheck size={24} />
          <div>
            <strong>Hermes Control</strong>
            <span>{t('brand.subtitle')}</span>
          </div>
        </div>

        <nav className="nav-list" aria-label="Main sections">
          {navigationSections.map((section) => {
            const Icon = navIcons[section.id];
            const label = t(navLabelKeys[section.id]);
            return (
              <button
                key={section.id}
                className={section.id === selectedSection ? 'nav-item active' : 'nav-item'}
                onClick={() => setSelectedSection(section.id)}
                type="button"
                title={label}
              >
                <Icon size={18} />
                <span>{label}</span>
              </button>
            );
          })}
        </nav>
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">Phase8 GUI</p>
            <h1>{t('topbar.title')}</h1>
          </div>
          <button className="icon-button" onClick={refresh} type="button" title={t('topbar.refresh')}>
            <RefreshCw size={18} className={loading ? 'spin' : undefined} />
          </button>
        </header>

        <section className="status-strip" aria-label="Runtime status">
          <Metric icon={Activity} label={t('metric.overall')} value={model?.overallLabel ?? t('status.offline')} />
          <Metric icon={Route} label={t('metric.activeRoute')} value={model?.activeRoute ?? t('status.notLoaded')} />
          <Metric icon={Bot} label={t('metric.modelsReady')} value={model ? `${model.readyModels}/${model.totalModels}` : '0/0'} />
          <Metric icon={TerminalSquare} label={t('metric.wsl')} value={model?.wslState ?? t('status.unknown')} />
          <Metric
            icon={ShieldCheck}
            label={t('metric.hermes')}
            value={
              snapshot
                ? model?.hermesReachable
                  ? t('status.reachable')
                  : t('status.unreachable')
                : t('status.unknown')
            }
          />
        </section>

        <section className="main-grid">
          <section className="primary-pane" aria-label="Selected control surface">
            {selectedSection === 'dashboard' && <Dashboard snapshot={snapshot} statusMessage={statusMessage} t={t} />}
            {selectedSection === 'route' && (
              <RoutePane
                t={t}
                options={routeOptions}
                selectedProfileId={selectedProfileId}
                setSelectedProfileId={setSelectedProfileId}
                previewSelectedRoute={previewSelectedRoute}
                previewRollback={previewRollback}
                executeSelectedRoute={executeSelectedRoute}
                executeRollback={executeRollback}
                routePreview={routePreview}
                routeMessage={routeMessage}
                routeBusy={routeBusy}
                confirmationPrompt={confirmationPrompt}
                confirmationCode={confirmationCode}
                setConfirmationCode={setConfirmationCode}
                confirmPendingOperation={confirmPendingOperation}
                cancelPendingOperation={cancelPendingOperation}
              />
            )}
            {selectedSection === 'models' && (
              <ModelsPane
                t={t}
                snapshot={snapshot}
                actionOptions={modelActionOptions}
                selectedModelId={selectedModelId}
                setSelectedModelId={setSelectedModelId}
                selectedAction={selectedModelAction}
                setSelectedAction={setSelectedModelAction}
                previewAction={previewSelectedModelAction}
                executeAction={executeSelectedModelAction}
                modelResponse={modelResponse}
                modelMessage={modelMessage}
                modelBusy={modelBusy}
                confirmationPrompt={modelConfirmationPrompt}
                confirmationCode={modelConfirmationCode}
                setConfirmationCode={setModelConfirmationCode}
                confirmPendingOperation={confirmPendingModelOperation}
                cancelPendingOperation={cancelPendingModelOperation}
              />
            )}
            {selectedSection === 'runtime' && (
              <RuntimePane
                t={t}
                snapshot={snapshot}
                wslOptions={wslActionOptions}
                hermesOptions={hermesActionOptions}
                openWebUiOptions={openWebUiActionOptions}
                selectedWslAction={selectedWslAction}
                setSelectedWslAction={setSelectedWslAction}
                selectedHermesAction={selectedHermesAction}
                setSelectedHermesAction={setSelectedHermesAction}
                selectedOpenWebUiAction={selectedOpenWebUiAction}
                setSelectedOpenWebUiAction={setSelectedOpenWebUiAction}
                previewRuntimeAction={previewSelectedRuntimeAction}
                executeRuntimeAction={executeSelectedRuntimeAction}
                runtimeResponse={runtimeResponse}
                runtimeMessage={runtimeMessage}
                runtimeBusy={runtimeBusy}
                confirmationPrompt={runtimeConfirmationPrompt}
                confirmationCode={runtimeConfirmationCode}
                setConfirmationCode={setRuntimeConfirmationCode}
                confirmPendingOperation={confirmPendingRuntimeOperation}
                cancelPendingOperation={cancelPendingRuntimeOperation}
              />
            )}
            {selectedSection === 'logs' && (
              <LogsPane
                t={t}
                targets={logTargets}
                selectedTarget={selectedLogTarget}
                setSelectedTarget={setSelectedLogTarget}
                tailSize={logTailSize}
                setTailSize={setLogTailSize}
                logFilter={logFilter}
                setLogFilter={setLogFilter}
                refreshLogs={refreshLogs}
                logTail={logTail}
                logMessage={logMessage}
              />
            )}
            {selectedSection === 'audit' && (
              <AuditPane
                t={t}
                snapshot={snapshot}
                riskFilter={auditRiskFilter}
                setRiskFilter={setAuditRiskFilter}
                requesterFilter={auditRequesterFilter}
                setRequesterFilter={setAuditRequesterFilter}
                queryFilter={auditQueryFilter}
                setQueryFilter={setAuditQueryFilter}
              />
            )}
            {selectedSection === 'info' && <InfoPane t={t} />}
            {selectedSection === 'settings' && (
              <SettingsPane
                settings={connectionSettings}
                setSettings={setConnectionSettings}
                settingsMessage={settingsMessage}
                settingsBusy={settingsBusy}
                desktopRuntime={desktopRuntime}
                language={language}
                setLanguage={setLanguage}
                t={t}
                refreshSettings={refreshConnectionSettings}
                saveSettings={saveConnectionSettings}
                testConnection={testConnectionSettings}
              />
            )}
          </section>
        </section>
      </section>
    </main>
  );
}

type RuntimeTarget = 'wsl' | 'hermes' | 'openwebui';

function previewRuntimeActionForTarget(
  target: RuntimeTarget,
  wslAction: WslActionId,
  hermesAction: HermesActionId,
  openWebUiAction: OpenWebUiActionId,
): Promise<OperationResponse> {
  if (target === 'wsl') {
    return previewWslAction(wslAction);
  }
  if (target === 'hermes') {
    return previewHermesAction(hermesAction);
  }
  return previewOpenWebUiAction(openWebUiAction);
}

function executeRuntimeActionForTarget(
  target: RuntimeTarget,
  wslAction: WslActionId,
  hermesAction: HermesActionId,
  openWebUiAction: OpenWebUiActionId,
): Promise<OperationResponse> {
  if (target === 'wsl') {
    return executeWslAction(wslAction);
  }
  if (target === 'hermes') {
    return executeHermesAction(hermesAction);
  }
  return executeOpenWebUiAction(openWebUiAction);
}

function readLanguageSetting(): LanguageId {
  if (typeof window === 'undefined') {
    return DEFAULT_LANGUAGE;
  }

  return normalizeLanguage(window.localStorage.getItem(LANGUAGE_STORAGE_KEY));
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

function Dashboard({
  snapshot,
  statusMessage,
  t,
}: {
  snapshot: GuiDashboardSnapshot | null;
  statusMessage: string;
  t: ReturnType<typeof createTranslator>;
}) {
  return (
    <>
      <PanelHeader title={t('dashboard.title')} note={statusMessage} />
      <div className="detail-table">
        <Row label={t('dashboard.hermesHealth')} value={snapshot?.status.hermes.message ?? t('status.waitingForDaemon')} />
        <Row label={t('dashboard.hermesUrl')} value={snapshot?.status.hermes.url ?? 'http://127.0.0.1:8642/health'} />
        <Row label={t('dashboard.lastKnownGoodRoute')} value={snapshot?.active_route.last_known_good_profile_id ?? t('status.notLoaded')} />
        <Row label={t('dashboard.stateStores')} value={buildStateStoreSummary(snapshot?.status.state, t)} />
      </div>
    </>
  );
}

function RoutePane({
  t,
  options,
  selectedProfileId,
  setSelectedProfileId,
  previewSelectedRoute,
  previewRollback,
  executeSelectedRoute,
  executeRollback,
  routePreview,
  routeMessage,
  routeBusy,
  confirmationPrompt,
  confirmationCode,
  setConfirmationCode,
  confirmPendingOperation,
  cancelPendingOperation,
}: {
  t: ReturnType<typeof createTranslator>;
  options: ReturnType<typeof buildRouteOptions>;
  selectedProfileId: string;
  setSelectedProfileId: (value: string) => void;
  previewSelectedRoute: () => void;
  previewRollback: () => void;
  executeSelectedRoute: () => void;
  executeRollback: () => void;
  routePreview: OperationResponse | null;
  routeMessage: string;
  routeBusy: boolean;
  confirmationPrompt: ReturnType<typeof buildConfirmationPrompt>;
  confirmationCode: string;
  setConfirmationCode: (value: string) => void;
  confirmPendingOperation: () => void;
  cancelPendingOperation: () => void;
}) {
  return (
    <>
      <PanelHeader title={t('route.title')} note={t('route.note')} />
      <div className="action-row">
        <label>
          <span>{t('route.profile')}</span>
          <select value={selectedProfileId} onChange={(event) => setSelectedProfileId(event.target.value)}>
            <option value="">{t('route.selectProfile')}</option>
            {options.map((option) => (
              <option key={option.id} value={option.id}>
                {option.label} / {option.kind}
              </option>
            ))}
          </select>
        </label>
        <button type="button" onClick={previewSelectedRoute} disabled={routeBusy} title={t('route.preview')}>
          <Search size={16} />
          <span>{t('route.preview')}</span>
        </button>
        <button type="button" onClick={executeSelectedRoute} disabled={routeBusy} title={t('route.switch')}>
          <Play size={16} />
          <span>{t('route.switch')}</span>
        </button>
        <button type="button" onClick={previewRollback} disabled={routeBusy} title={t('route.previewRollback')}>
          <Search size={16} />
          <span>{t('route.previewRollback')}</span>
        </button>
        <button
          className="danger-button"
          type="button"
          onClick={executeRollback}
          disabled={routeBusy}
          title={t('route.rollback')}
        >
          <RotateCcw size={16} />
          <span>{t('route.rollback')}</span>
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
              option.baseUrl,
              option.defaultModel,
              option.accountSummary,
              option.secretEnvKey,
              option.runtimeEnvKeys.length ? `env: ${option.runtimeEnvKeys.join(', ')}` : '',
              option.isActive ? t('route.active') : '',
              option.isLastKnownGood ? t('route.lastKnownGood') : '',
            ]
              .filter(Boolean)
              .join(' / ')}
          />
        ))}
      </div>
      {routePreview && <OperationPreview response={routePreview} t={t} />}
      {confirmationPrompt && (
        <ConfirmationSheet
          t={t}
          prompt={confirmationPrompt}
          code={confirmationCode}
          setCode={setConfirmationCode}
          confirm={confirmPendingOperation}
          cancel={cancelPendingOperation}
          busy={routeBusy}
        />
      )}
    </>
  );
}

function ModelsPane({
  t,
  snapshot,
  actionOptions,
  selectedModelId,
  setSelectedModelId,
  selectedAction,
  setSelectedAction,
  previewAction,
  executeAction,
  modelResponse,
  modelMessage,
  modelBusy,
  confirmationPrompt,
  confirmationCode,
  setConfirmationCode,
  confirmPendingOperation,
  cancelPendingOperation,
}: {
  t: ReturnType<typeof createTranslator>;
  snapshot: GuiDashboardSnapshot | null;
  actionOptions: ReturnType<typeof buildModelActionOptions>;
  selectedModelId: string;
  setSelectedModelId: (value: string) => void;
  selectedAction: ModelActionId;
  setSelectedAction: (value: ModelActionId) => void;
  previewAction: () => void;
  executeAction: () => void;
  modelResponse: OperationResponse | null;
  modelMessage: string;
  modelBusy: boolean;
  confirmationPrompt: ReturnType<typeof buildConfirmationPrompt>;
  confirmationCode: string;
  setConfirmationCode: (value: string) => void;
  confirmPendingOperation: () => void;
  cancelPendingOperation: () => void;
}) {
  return (
    <>
      <PanelHeader title={t('models.title')} note={t('models.note')} />
      <div className="action-row">
        <label>
          <span>{t('models.model')}</span>
          <select value={selectedModelId} onChange={(event) => setSelectedModelId(event.target.value)}>
            <option value="">{t('models.selectModel')}</option>
            {(snapshot?.models ?? []).map((model) => (
              <option key={model.variant_id} value={model.variant_id}>
                {model.variant_id}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>{t('models.action')}</span>
          <select
            value={selectedAction}
            onChange={(event) => setSelectedAction(event.target.value as ModelActionId)}
          >
            {actionOptions.map((action) => (
              <option key={action.id} value={action.id}>
                {action.label} / {action.riskHint}
              </option>
            ))}
          </select>
        </label>
        <button type="button" onClick={previewAction} disabled={modelBusy} title={t('models.preview')}>
          <Search size={16} />
          <span>{t('models.preview')}</span>
        </button>
        <button type="button" onClick={executeAction} disabled={modelBusy} title={t('models.run')}>
          <Play size={16} />
          <span>{t('models.run')}</span>
        </button>
      </div>
      <p className="inline-status">{modelMessage}</p>
      <div className="model-list">
        {(snapshot?.models ?? []).map((model) => (
          <button
            className={model.variant_id === selectedModelId ? 'model-row selected' : 'model-row'}
            key={model.variant_id}
            onClick={() => setSelectedModelId(model.variant_id)}
            type="button"
            title={`${t('models.selectModel')} ${model.variant_id}`}
          >
            <span className={model.ready ? 'dot ok' : 'dot down'} />
            <div>
              <strong>{model.variant_id}</strong>
              <span>{model.served_model_name}</span>
              {model.model_root && <span>{`${t('models.modelRoot')}: ${model.model_root}`}</span>}
            </div>
            <code>{model.endpoint.url}</code>
          </button>
        ))}
      </div>
      {modelResponse && <OperationPreview response={modelResponse} t={t} />}
      {confirmationPrompt && (
        <ConfirmationSheet
          t={t}
          prompt={confirmationPrompt}
          code={confirmationCode}
          setCode={setConfirmationCode}
          confirm={confirmPendingOperation}
          cancel={cancelPendingOperation}
          busy={modelBusy}
        />
      )}
    </>
  );
}

function RuntimePane({
  t,
  snapshot,
  wslOptions,
  hermesOptions,
  openWebUiOptions,
  selectedWslAction,
  setSelectedWslAction,
  selectedHermesAction,
  setSelectedHermesAction,
  selectedOpenWebUiAction,
  setSelectedOpenWebUiAction,
  previewRuntimeAction,
  executeRuntimeAction,
  runtimeResponse,
  runtimeMessage,
  runtimeBusy,
  confirmationPrompt,
  confirmationCode,
  setConfirmationCode,
  confirmPendingOperation,
  cancelPendingOperation,
}: {
  t: ReturnType<typeof createTranslator>;
  snapshot: GuiDashboardSnapshot | null;
  wslOptions: ReturnType<typeof buildWslActionOptions>;
  hermesOptions: ReturnType<typeof buildHermesActionOptions>;
  openWebUiOptions: ReturnType<typeof buildOpenWebUiActionOptions>;
  selectedWslAction: WslActionId;
  setSelectedWslAction: (value: WslActionId) => void;
  selectedHermesAction: HermesActionId;
  setSelectedHermesAction: (value: HermesActionId) => void;
  selectedOpenWebUiAction: OpenWebUiActionId;
  setSelectedOpenWebUiAction: (value: OpenWebUiActionId) => void;
  previewRuntimeAction: (target: RuntimeTarget) => void;
  executeRuntimeAction: (target: RuntimeTarget) => void;
  runtimeResponse: OperationResponse | null;
  runtimeMessage: string;
  runtimeBusy: boolean;
  confirmationPrompt: ReturnType<typeof buildConfirmationPrompt>;
  confirmationCode: string;
  setConfirmationCode: (value: string) => void;
  confirmPendingOperation: () => void;
  cancelPendingOperation: () => void;
}) {
  return (
    <>
      <PanelHeader title={t('runtime.title')} note={t('runtime.note')} />
      <div className="runtime-actions">
        <RuntimeActionGroup
          t={t}
          title="WSL"
          options={wslOptions}
          selected={selectedWslAction}
          setSelected={setSelectedWslAction}
          preview={() => previewRuntimeAction('wsl')}
          run={() => executeRuntimeAction('wsl')}
          busy={runtimeBusy}
        />
        <RuntimeActionGroup
          t={t}
          title="Hermes"
          options={hermesOptions}
          selected={selectedHermesAction}
          setSelected={setSelectedHermesAction}
          preview={() => previewRuntimeAction('hermes')}
          run={() => executeRuntimeAction('hermes')}
          busy={runtimeBusy}
        />
        <RuntimeActionGroup
          t={t}
          title={t('runtime.openwebui')}
          options={openWebUiOptions}
          selected={selectedOpenWebUiAction}
          setSelected={setSelectedOpenWebUiAction}
          preview={() => previewRuntimeAction('openwebui')}
          run={() => executeRuntimeAction('openwebui')}
          busy={runtimeBusy}
        />
      </div>
      <p className="inline-status">{runtimeMessage}</p>
      <div className="detail-table">
        <Row label={t('runtime.wslDistro')} value={snapshot?.status.wsl?.name ?? t('status.notLoaded')} />
        <Row label={t('runtime.wslState')} value={snapshot?.status.wsl?.state ?? t('status.unknown')} />
        <Row label={t('runtime.hermesReachable')} value={snapshot?.status.hermes.reachable ? t('settings.yes') : t('status.unknown')} />
      </div>
      {runtimeResponse && <OperationPreview response={runtimeResponse} t={t} />}
      {confirmationPrompt && (
        <ConfirmationSheet
          t={t}
          prompt={confirmationPrompt}
          code={confirmationCode}
          setCode={setConfirmationCode}
          confirm={confirmPendingOperation}
          cancel={cancelPendingOperation}
          busy={runtimeBusy}
        />
      )}
    </>
  );
}

function RuntimeActionGroup<T extends string>({
  t,
  title,
  options,
  selected,
  setSelected,
  preview,
  run,
  busy,
}: {
  t: ReturnType<typeof createTranslator>;
  title: string;
  options: Array<{ id: T; label: string; riskHint: string }>;
  selected: T;
  setSelected: (value: T) => void;
  preview: () => void;
  run: () => void;
  busy: boolean;
}) {
  return (
    <section className="runtime-action-group" aria-label={`${title} actions`}>
      <h3>{title}</h3>
      <div className="action-row">
        <label>
          <span>{t('runtime.action')}</span>
          <select value={selected} onChange={(event) => setSelected(event.target.value as T)}>
            {options.map((option) => (
              <option key={option.id} value={option.id}>
                {option.label} / {option.riskHint}
              </option>
            ))}
          </select>
        </label>
        <button type="button" onClick={preview} disabled={busy} title={`${t('runtime.preview')} ${title}`}>
          <Search size={16} />
          <span>{t('runtime.preview')}</span>
        </button>
        <button type="button" onClick={run} disabled={busy} title={`${t('runtime.run')} ${title}`}>
          <Play size={16} />
          <span>{t('runtime.run')}</span>
        </button>
      </div>
    </section>
  );
}

function LogsPane({
  t,
  targets,
  selectedTarget,
  setSelectedTarget,
  tailSize,
  setTailSize,
  logFilter,
  setLogFilter,
  refreshLogs,
  logTail,
  logMessage,
}: {
  t: ReturnType<typeof createTranslator>;
  targets: ReturnType<typeof buildLogTargets>;
  selectedTarget: LogTargetId;
  setSelectedTarget: (value: LogTargetId) => void;
  tailSize: number;
  setTailSize: (value: number) => void;
  logFilter: string;
  setLogFilter: (value: string) => void;
  refreshLogs: () => void;
  logTail: GuiLogTail | null;
  logMessage: string;
}) {
  const visibleLines = buildFilteredLogLines(logTail?.lines ?? [], logFilter);

  return (
    <>
      <PanelHeader title={t('logs.title')} note={t('logs.note')} />
      <div className="action-row">
        <label>
          <span>{t('logs.target')}</span>
          <select
            value={selectedTarget}
            onChange={(event) => setSelectedTarget(event.target.value as LogTargetId)}
          >
            {targets.map((target) => (
              <option key={target.id} value={target.id}>
                {target.label}
              </option>
            ))}
          </select>
        </label>
        <label className="compact-label">
          <span>{t('logs.tail')}</span>
          <input
            min={1}
            max={1000}
            onChange={(event) => setTailSize(Number(event.target.value))}
            type="number"
            value={tailSize}
          />
        </label>
        <label>
          <span>{t('logs.filter')}</span>
          <input
            onChange={(event) => setLogFilter(event.target.value)}
            placeholder={t('logs.filterPlaceholder')}
            value={logFilter}
          />
        </label>
        <button type="button" onClick={refreshLogs}>
          {t('logs.tailLogs')}
        </button>
      </div>
      <p className="inline-status">{logMessage} / {t('logs.showing')} {visibleLines.length} {t('logs.lines')}</p>
      <pre className="log-preview">
        {(visibleLines.length ? visibleLines : [t('logs.empty')]).join('\n')}
      </pre>
    </>
  );
}

function AuditPane({
  t,
  snapshot,
  riskFilter,
  setRiskFilter,
  requesterFilter,
  setRequesterFilter,
  queryFilter,
  setQueryFilter,
}: {
  t: ReturnType<typeof createTranslator>;
  snapshot: GuiDashboardSnapshot | null;
  riskFilter: string;
  setRiskFilter: (value: string) => void;
  requesterFilter: string;
  setRequesterFilter: (value: string) => void;
  queryFilter: string;
  setQueryFilter: (value: string) => void;
}) {
  const auditEvents = snapshot?.audit ?? [];
  const riskOptions = buildAuditRiskOptions(auditEvents);
  const visibleEvents = buildFilteredAuditEvents(auditEvents, {
    riskLevel: riskFilter,
    requester: requesterFilter,
    query: queryFilter,
  });

  return (
    <>
      <PanelHeader title={t('audit.title')} note={t('audit.note')} />
      <div className="action-row">
        <label>
          <span>{t('audit.risk')}</span>
          <select value={riskFilter} onChange={(event) => setRiskFilter(event.target.value)}>
            {riskOptions.map((risk) => (
              <option key={risk} value={risk}>
                {translateAuditRiskFilter(risk, t)}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>{t('audit.requester')}</span>
          <input
            onChange={(event) => setRequesterFilter(event.target.value)}
            placeholder={t('audit.requesterPlaceholder')}
            value={requesterFilter}
          />
        </label>
        <label>
          <span>{t('audit.search')}</span>
          <input
            onChange={(event) => setQueryFilter(event.target.value)}
            placeholder={t('audit.searchPlaceholder')}
            value={queryFilter}
          />
        </label>
      </div>
      <p className="inline-status">{t('audit.showing')} {visibleEvents.length} {t('audit.events')}</p>
      <div className="audit-list">
        {visibleEvents.map((event) => (
          <div className="audit-row" key={event.id}>
            <History size={16} />
            <span>
              {event.action}
              <small>{event.requester_channel}:{event.requester_user_id}</small>
            </span>
            <strong>{translateRiskLevel(event.risk_level, t)}</strong>
          </div>
        ))}
      </div>
    </>
  );
}

function SettingsPane({
  settings,
  setSettings,
  settingsMessage,
  settingsBusy,
  desktopRuntime,
  language,
  setLanguage,
  t,
  refreshSettings,
  saveSettings,
  testConnection,
}: {
  settings: GuiConnectionSettings;
  setSettings: (value: GuiConnectionSettings) => void;
  settingsMessage: string;
  settingsBusy: boolean;
  desktopRuntime: boolean;
  language: LanguageId;
  setLanguage: (value: LanguageId) => void;
  t: ReturnType<typeof createTranslator>;
  refreshSettings: () => void;
  saveSettings: () => void;
  testConnection: () => void;
}) {
  const model = buildSettingsViewModel(settings, desktopRuntime, t);
  const updateSetting = (field: keyof GuiConnectionSettings, value: string) => {
    setSettings({ ...settings, [field]: value });
  };

  return (
    <>
      <PanelHeader title={t('settings.title')} note={t('settings.note')} />
      <div className="settings-layout">
        <section className="settings-form" aria-label={t('settings.connectionLabel')}>
          <div className="settings-grid">
            <label>
              <span>{t('settings.language')}</span>
              <select
                value={language}
                onChange={(event) => setLanguage(event.target.value as LanguageId)}
              >
                {languageOptions.map((option) => (
                  <option key={option.id} value={option.id}>
                    {option.label}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span>{t('settings.daemonUrl')}</span>
              <input
                value={settings.daemonUrl}
                onChange={(event) => updateSetting('daemonUrl', event.target.value)}
                disabled={!model.canEditConnection}
                spellCheck={false}
              />
            </label>
            <label>
              <span>{t('settings.operatorId')}</span>
              <input
                value={settings.operatorId}
                onChange={(event) => updateSetting('operatorId', event.target.value)}
                disabled={!model.canEditConnection}
                spellCheck={false}
              />
            </label>
            <label className="wide-field">
              <span>{t('settings.apiToken')}</span>
              <input
                value={settings.apiToken}
                onChange={(event) => updateSetting('apiToken', event.target.value)}
                disabled={!model.canEditConnection}
                type="password"
                autoComplete="off"
                spellCheck={false}
              />
            </label>
          </div>
          <div className="button-group">
            <button type="button" onClick={refreshSettings} disabled={settingsBusy} title={t('settings.reload')}>
              <RefreshCw size={16} />
              <span>{t('settings.reload')}</span>
            </button>
            <button type="button" onClick={saveSettings} disabled={settingsBusy} title={t('settings.save')}>
              <Check size={16} />
              <span>{model.canEditConnection ? t('settings.save') : t('settings.readEnv')}</span>
            </button>
            <button type="button" onClick={testConnection} disabled={settingsBusy} title={t('settings.testConnection')}>
              <Play size={16} />
              <span>{t('settings.testConnection')}</span>
            </button>
          </div>
          <p className="inline-status">{settingsMessage}</p>
        </section>
        <div className="detail-table">
          <Row label={t('settings.mode')} value={model.modeLabel} />
          <Row label={t('settings.storage')} value={model.storageLabel} />
          <Row label={t('settings.daemonUrl')} value={model.daemonUrl} />
          <Row label={t('settings.operator')} value={model.operatorId} />
          <Row label={t('settings.token')} value={model.tokenLabel} />
          <Row label={t('settings.tokenConfigured')} value={model.tokenConfigured ? t('settings.yes') : t('settings.no')} />
        </div>
      </div>
    </>
  );
}

function InfoPane({ t }: { t: ReturnType<typeof createTranslator> }) {
  return (
    <>
      <PanelHeader title={t('info.title')} note={t('info.note')} />
      <div className="info-layout">
        <section className="info-section" aria-label={t('info.boundaryLabel')}>
          <p className="eyebrow">{t('boundary.eyebrow')}</p>
          <h3>{t('boundary.title')}</h3>
          <p>{t('boundary.copy')}</p>
          <div className="boundary-list">
            <span>core:default</span>
            <span>Authorization: Bearer</span>
            <span>Requester: gui</span>
          </div>
        </section>
        <section className="info-section" aria-label={t('info.openwebuiChain')}>
          <p className="eyebrow">{t('info.openwebuiChain')}</p>
          <h3>{t('runtime.openwebui')}</h3>
          <p>{t('info.openwebuiChainCopy')}</p>
          <div className="boundary-list">
            <span>/v1/openwebui/action</span>
            <span>hermes-control-openwebui-refresh.sh</span>
            <span>hermes-control-openwebui-stop.sh</span>
            <span>hermes-control-openwebui-status.sh</span>
          </div>
        </section>
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

function OperationPreview({
  response,
  t,
}: {
  response: OperationResponse;
  t: ReturnType<typeof createTranslator>;
}) {
  const title = response.dry_run ? t('operation.dryRunPreview') : t('operation.response');

  return (
    <section className="operation-preview" aria-label={title}>
      <PanelHeader title={title} note={response.summary} />
      <div className="detail-table">
        <Row label={t('operation.status')} value={response.status} />
        <Row label={t('operation.risk')} value={translateRiskLevel(response.risk, t)} />
        <Row label={t('operation.dryRun')} value={String(response.dry_run)} />
        <Row label={t('operation.confirmation')} value={response.confirmation_id ?? t('operation.notRequired')} />
      </div>
      {!!response.commands?.length && (
        <pre className="command-preview">
          {response.commands.map((command) => {
            const envKeys = Object.keys(command.env ?? {}).sort();
            const envPreview = envKeys.length ? ` env:${envKeys.join(',')}` : '';
            return `${command.program} ${command.args.join(' ')}${envPreview}`;
          }).join('\n')}
        </pre>
      )}
      {response.output && <pre className="command-preview">{response.output}</pre>}
    </section>
  );
}

function ConfirmationSheet({
  t,
  prompt,
  code,
  setCode,
  confirm,
  cancel,
  busy,
}: {
  t: ReturnType<typeof createTranslator>;
  prompt: NonNullable<ReturnType<typeof buildConfirmationPrompt>>;
  code: string;
  setCode: (value: string) => void;
  confirm: () => void;
  cancel: () => void;
  busy: boolean;
}) {
  return (
    <section className="confirmation-sheet" aria-label={t('confirm.title')}>
      <PanelHeader title={t('confirm.title')} note={prompt.summary} />
      <div className="detail-table">
        <Row label={t('operation.risk')} value={translateRiskLevel(prompt.risk, t)} />
        <Row label={t('confirm.id')} value={prompt.confirmationId} />
        <Row label={t('confirm.expiresAt')} value={prompt.expiresAt} />
      </div>
      <label className="confirm-code">
        <span>{t('confirm.code')}</span>
        <input
          value={code}
          onChange={(event) => setCode(event.target.value)}
          placeholder={prompt.codeHint}
          autoComplete="off"
        />
      </label>
      <div className="button-group">
        <button type="button" onClick={confirm} disabled={busy} title={t('confirm.confirm')}>
          <Check size={16} />
          <span>{t('confirm.confirm')}</span>
        </button>
        <button className="danger-button" type="button" onClick={cancel} disabled={busy} title={t('confirm.cancel')}>
          <X size={16} />
          <span>{t('confirm.cancel')}</span>
        </button>
      </div>
    </section>
  );
}

function formatRequestError(error: unknown, fallback: string, conflictMessage?: string): string {
  if (!(error instanceof Error)) {
    return fallback;
  }

  if (conflictMessage && error.message.includes('409 Conflict')) {
    return conflictMessage;
  }

  return error.message || fallback;
}
