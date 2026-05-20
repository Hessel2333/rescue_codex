export type AppInfo = {
  defaultCodexRoot: string;
  databasePath: string;
  sessionIndexPath: string;
  platform: string;
};

export type DashboardRangePreset = "7d" | "30d" | "90d" | "365d" | "all" | "custom";
export type DashboardGranularity = "day" | "week" | "month" | "year";
export type DashboardSection =
  | "overview"
  | "performance"
  | "workflow"
  | "search"
  | "projects"
  | "correlations"
  | "imports"
  | "settings"
  | "all";

export type DashboardFilters = {
  preset?: DashboardRangePreset;
  granularity?: DashboardGranularity;
  dateFrom?: string;
  dateTo?: string;
  project?: string;
};

export type DashboardScope = {
  preset: DashboardRangePreset;
  granularity: DashboardGranularity;
  dateFrom: string;
  dateTo: string;
  availableFrom?: string | null;
  availableTo?: string | null;
  totalDays: number;
};

export type AccountInfo = {
  maskedEmail?: string | null;
  planType?: string | null;
  maskedAccountUserId?: string | null;
  currentModel?: string | null;
  currentReasoningEffort?: string | null;
  currentSpeedTier?: string | null;
  lastRefresh?: string | null;
};

export type DashboardOverview = {
  totalSessions: number;
  totalQuestions: number;
  activeDays: number;
  sessionsLast7Days: number;
  sessionsLast30Days: number;
  avgDurationSec: number;
  avgTurnCount: number;
  totalToolCalls: number;
  avgFirstResponseSec: number;
  avgTurnCompletionSec: number;
};

export type ChartDatum = {
  label: string;
  value: number;
};

export type BreakdownDatum = {
  bucket: string;
  category: string;
  value: number;
};

export type ToolMetricDatum = {
  label: string;
  total: number;
  success: number;
  failure: number;
  avgDurationSec: number;
};

export type TokenUsageSummary = {
  inputTokens: number;
  outputTokens: number;
  cachedInputTokens: number;
  reasoningOutputTokens: number;
  totalTokens: number;
};

export type RankedTurnRecord = {
  sessionId: string;
  project: string;
  threadTitle?: string | null;
  promptPreview?: string | null;
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  cachedInputTokens: number;
  reasoningOutputTokens: number;
  firstResponseSec?: number | null;
  completionSec?: number | null;
  timestamp?: string | null;
};

export type ProjectSummary = {
  label: string;
  sessionCount: number;
  questionCount: number;
  totalTokens: number;
  contextCompactions: number;
  avgFirstResponseSec: number;
  avgCompletionSec: number;
  maxParallelWindows: number;
};

export type ProjectWindowRecord = {
  sessionId: string;
  project: string;
  threadTitle?: string | null;
  startedAt?: string | null;
  updatedAt?: string | null;
  durationSec: number;
  turnCount: number;
  totalTokens: number;
  questionCount: number;
  toolCallCount: number;
};

export type CorrelationDatum = {
  bucket: string;
  sampleCount: number;
  avgFirstResponseSec: number;
  avgCompletionSec: number;
  avgTotalTokens: number;
  avgTokenRate: number;
};

export type ScatterDatum = {
  x: number;
  completionSec: number;
  totalTokens: number;
  firstResponseSec: number;
  tokenRate: number;
  label: string;
  detail: string;
};

export type ActivityPoint = {
  date: string;
  sessions: number;
  questions: number;
  avgFirstResponseSec: number;
  avgTurnCompletionSec: number;
};

export type RecentImport = {
  id: string;
  sourceLabel: string;
  status: string;
  mode: string;
  filesTotal: number;
  filesSuccess: number;
  filesFailed: number;
  warningsCount: number;
  errorsCount: number;
  startedAt: string;
  finishedAt?: string | null;
};

export type ImportIssue = {
  id: string;
  severity: string;
  code: string;
  message: string;
  lineNo?: number | null;
  rawExcerpt?: string | null;
  createdAt: string;
  path?: string | null;
};

export type SessionSummary = {
  id: string;
  threadTitle?: string | null;
  cwd?: string | null;
  source?: string | null;
  updatedAt?: string | null;
  startedAt?: string | null;
  durationSec: number;
  userMessageCount: number;
  assistantMessageCount: number;
  toolCallCount: number;
  turnCount: number;
  warningCount: number;
  firstUserMessage?: string | null;
};

export type SessionMessage = {
  id: string;
  turnId?: string | null;
  role?: string | null;
  kind: string;
  text?: string | null;
  ts?: string | null;
  toolName?: string | null;
  phase?: string | null;
  imageUrls?: string[];
  image_urls?: string[];
};

export type SessionDetail = {
  session: SessionSummary;
  messages: SessionMessage[];
};

export type DashboardSummary = {
  scope: DashboardScope;
  overview: DashboardOverview;
  activity: ActivityPoint[];
  dailyActivity: ActivityPoint[];
  heatmapActivity: ActivityPoint[];
  projectOptions: string[];
  selectedProject?: string | null;
  projectTimeline: BreakdownDatum[];
  projectSummaries: ProjectSummary[];
  projectParallelism: ChartDatum[];
  durationBuckets: ChartDatum[];
  questionHours: ChartDatum[];
  firstTokenBuckets: ChartDatum[];
  completionBuckets: ChartDatum[];
  tokenUsage: TokenUsageSummary;
  topTokenTurns: RankedTurnRecord[];
  slowestTurns: RankedTurnRecord[];
  projectWindows: ProjectWindowRecord[];
  toolTypes: ChartDatum[];
  toolMetrics: ToolMetricDatum[];
  modelUsage: ChartDatum[];
  modelTimeline: BreakdownDatum[];
  reasoningEfforts: ChartDatum[];
  reasoningTimeline: BreakdownDatum[];
  speedTiers: ChartDatum[];
  speedTimeline: BreakdownDatum[];
  topPromptTerms: ChartDatum[];
  promptLengthBuckets: ChartDatum[];
  promptComposition: ChartDatum[];
  transportSignals: ChartDatum[];
  transportTimeline: BreakdownDatum[];
  interruptionTimeline: BreakdownDatum[];
  workspaceSwitches: ChartDatum[];
  workspaceTimeline: BreakdownDatum[];
  hourlyCorrelations: CorrelationDatum[];
  weekdayCorrelations: CorrelationDatum[];
  promptLengthCorrelations: CorrelationDatum[];
  toolLoadCorrelations: CorrelationDatum[];
  contextLoadCorrelations: CorrelationDatum[];
  hourlyCorrelationScatter: ScatterDatum[];
  weekdayCorrelationScatter: ScatterDatum[];
  promptLengthCorrelationScatter: ScatterDatum[];
  toolLoadCorrelationScatter: ScatterDatum[];
  contextLoadCorrelationScatter: ScatterDatum[];
  searchKeywords: ChartDatum[];
  searchHours: ChartDatum[];
  topCwds: ChartDatum[];
  topSources: ChartDatum[];
  recentImports: RecentImport[];
  recentSessions: SessionSummary[];
  recentIssues: ImportIssue[];
  appInfo: AppInfo;
  accountInfo: AccountInfo;
};

export type SessionListFilters = {
  query?: string;
  cwd?: string;
  source?: string;
  dateFrom?: string;
  dateTo?: string;
  limit?: number;
  offset?: number;
  sessionId?: string;
};

export type SessionListResponse = {
  total: number;
  items: SessionSummary[];
  selected?: SessionDetail | null;
};

export type ImportRunResult = {
  importId: string;
  sourceLabel: string;
  rootPath: string;
  status: string;
  filesTotal: number;
  filesSuccess: number;
  filesFailed: number;
  warningsCount: number;
  errorsCount: number;
  issues: ImportIssue[];
};

export type ExportFormat = "csv" | "json" | "markdown";
export type ExportKind = "dashboard" | "sessions";

export type ExportRequest = {
  kind: ExportKind;
  format: ExportFormat;
  path: string;
  filters?: SessionListFilters;
  dashboardFilters?: DashboardFilters;
};

export type ExportResult = {
  id: string;
  path: string;
  format: ExportFormat;
  bytesWritten: number;
};
