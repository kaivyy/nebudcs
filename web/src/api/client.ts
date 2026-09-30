/**
 * NebuDCS REST API Client.
 */

import {
  AbortSessionRequest,
  AdvancedControlStrategyDto,
  AlarmSnapshotDto,
  CommunicationsConfigDto,
  ConnectionTestResultDto,
  ControlModuleActionRequestDto,
  ControllerDetailDto,
  ControllerFaultInjectionRequestDto,
  ControllerSummaryDto,
  CreateProjectRequestDto,
  CreateTrainingSessionRequest,
  CrossReferenceResultDto,
  DomainPackageDetailDto,
  DomainPackageSummaryDto,
  EquipmentControlRequestDto,
  EvaluationReportDto,
  HalChannelDto,
  HalFaultRequestDto,
  HalMappingDto,
  HalSystemSummaryDto,
  IecCompileRequestDto,
  IecCompileResponseDto,
  IndustrialZoneDto,
  LoadSessionProjectRequest,
  NetworkDiagnosticsDto,
  PidModeDto,
  ProjectDetailDto,
  ProjectSummaryDto,
  ProjectValidationResponseDto,
  ProtocolConnectionDto,
  ProtocolDiagnosticsDto,
  RecordTraineeActionRequest,
  ReplayFrameDto,
  ReplaySessionSummaryDto,
  SaveProjectRequestDto,
  ScenarioAuthoringDto,
  ScenarioStatusDto,
  ScenarioSummaryDto,
  ScenarioValidationResultDto,
  ServerHealthDto,
  SimControlAction,
  StateSnapshotDto,
  TagMappingDto,
  TagValueDto,
  TagValueSampleDto,
  TemplateSummaryDto,
  TraineeActionDto,
  TrainingSessionDetailDto,
  TrainingSessionSummaryDto,
  ScenarioBranchDto,
  TrainingDebriefDto,
  CauseEffectMatrixDto,
  SifBypassRequestDto,
  SifDetailDto,
  SisStatusSummaryDto,
  AlarmSummaryDto,
  EquipmentDiagnosticsDto,
  FirstOutEventDto,
  ValidationExecutiveSummaryDto,
  ValidationCaseSummaryDto,
  ValidationCaseDetailDto,
  ValidationRunRequestDto,
  ValidationRunResponseDto,
  ReferenceDatasetDto,
  ModelValidationSummaryDto,
  ValidationManifestDto,
  ClusterTopologyDto,
  FailoverRequestDto,
  PromotionResultDto,
  FailureInjectionRequestDto,
  FailureInjectionStatusDto,
  ReplicationSnapshotDto,
  AuditEventDto,
  AuditSummaryDto,
  SubsystemObservabilityDto,
  DiagnosticItemDto,
  DiagnosticsSummaryDto,
  RestoreValidationDto,
  RestoreResultDto,
  SecurityStatusDto,
  PurdueZoneDto,
  PurdueConduitDto,
  UpdateConduitPolicyRequestDto,
  SecurityIncidentDto,
  AuditVerificationResultDto,
} from "../protocol/types";

import { authStore } from "../state/auth-store";

export interface ApiClientConfig {
  baseUrl?: string;
  authToken?: string;
}

export class ApiClient {
  private baseUrl: string;
  private authToken?: string;

  constructor(config: ApiClientConfig = {}) {
    this.baseUrl = config.baseUrl || "";
    this.authToken = config.authToken !== undefined ? config.authToken : authStore.getToken();
  }

  setAuthToken(token?: string) {
    this.authToken = token;
  }

  getAuthToken(): string | undefined {
    return this.authToken;
  }

  private async request<T>(
    endpoint: string,
    method: "GET" | "POST" | "PUT" | "DELETE" = "GET",
    body?: unknown
  ): Promise<T> {
    const headers: Record<string, string> = {
      Accept: "application/json",
    };

    if (this.authToken) {
      headers["Authorization"] = `Bearer ${this.authToken}`;
    }

    if (body !== undefined) {
      headers["Content-Type"] = "application/json";
    }

    const res = await fetch(`${this.baseUrl}${endpoint}`, {
      method,
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });

    if (!res.ok) {
      let errMessage = `HTTP ${res.status} ${res.statusText}`;
      try {
        const errJson = await res.json();
        if (errJson.error) {
          errMessage = errJson.error;
        } else if (errJson.message) {
          errMessage = errJson.message;
        }
      } catch {
        // Fallback to text status
      }
      throw new Error(errMessage);
    }

    return res.json() as Promise<T>;
  }

  async getHealth(): Promise<ServerHealthDto> {
    return this.request<ServerHealthDto>("/api/v1/health");
  }

  async getSnapshot(): Promise<StateSnapshotDto> {
    return this.request<StateSnapshotDto>("/api/v1/sim/snapshot");
  }

  async controlSim(cmd: SimControlAction): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/sim/control", "POST", cmd);
  }

  async listTags(): Promise<{ count: number; tags: TagValueSampleDto[] }> {
    return this.request<{ count: number; tags: TagValueSampleDto[] }>("/api/v1/tags");
  }

  async getTag(tagId: string): Promise<TagValueSampleDto> {
    return this.request<TagValueSampleDto>(`/api/v1/tags/${encodeURIComponent(tagId)}`);
  }

  async writeTag(tag: string, value: TagValueDto): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/tags/write", "POST", {
      tag,
      value,
    });
  }

  async listAlarms(): Promise<{ count: number; alarms: AlarmSnapshotDto[] }> {
    return this.request<{ count: number; alarms: AlarmSnapshotDto[] }>("/api/v1/alarms");
  }

  async getAlarmSummary(): Promise<AlarmSummaryDto> {
    return this.request<AlarmSummaryDto>("/api/v1/alarms/summary");
  }

  async listDiagnostics(): Promise<EquipmentDiagnosticsDto[]> {
    return this.request<EquipmentDiagnosticsDto[]>("/api/v1/diagnostics");
  }

  async getEquipmentDiagnostics(id: string): Promise<EquipmentDiagnosticsDto> {
    return this.request<EquipmentDiagnosticsDto>(`/api/v1/diagnostics/${encodeURIComponent(id)}`);
  }

  async getFirstOutEvent(runId: string = "active"): Promise<FirstOutEventDto> {
    return this.request<FirstOutEventDto>(`/api/v1/history/events/first-out?run_id=${encodeURIComponent(runId)}`);
  }

  async acknowledgeAlarm(alarmName: string): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/alarms/acknowledge", "POST", {
      alarm_name: alarmName,
    });
  }

  async shelveAlarm(
    alarmName: string,
    durationSeconds: number
  ): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/alarms/shelve", "POST", {
      alarm_name: alarmName,
      duration_seconds: durationSeconds,
    });
  }

  async resetProtection(blockName: string = "ESD-101"): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/protection/reset", "POST", {
      block_name: blockName,
    });
  }

  async setPidSetpoint(loopName: string, sp: number): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/control/pid/setpoint", "POST", {
      loop_name: loopName,
      sp,
    });
  }

  async setPidMode(loopName: string, mode: PidModeDto): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/control/pid/mode", "POST", {
      loop_name: loopName,
      mode,
    });
  }

  async setManualOutput(
    loopName: string,
    output: number
  ): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/control/pid/output", "POST", {
      loop_name: loopName,
      output,
    });
  }

  async getScenarioStatus(): Promise<ScenarioStatusDto> {
    return this.request<ScenarioStatusDto>("/api/v1/scenario/status");
  }

  async loadScenario(scenario: unknown): Promise<{ status: string; scenario_id?: string }> {
    return this.request<{ status: string; scenario_id?: string }>("/api/v1/scenario/load", "POST", scenario);
  }

  async startScenario(): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/scenario/start", "POST");
  }

  async pauseScenario(): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/scenario/pause", "POST");
  }

  async resumeScenario(): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/scenario/resume", "POST");
  }

  async abortScenario(reason = ""): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/scenario/abort", "POST", { reason });
  }

  async resetScenario(): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/scenario/reset", "POST");
  }

  async injectFault(fault: unknown): Promise<{ status: string; fault_id?: string }> {
    return this.request<{ status: string; fault_id?: string }>("/api/v1/scenario/fault/inject", "POST", fault);
  }

  async clearFault(faultId: string): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/scenario/fault/clear", "POST", { fault_id: faultId });
  }

  // -------------------------------------------------------------------------
  // Engineering Studio & Project Management Endpoints
  // -------------------------------------------------------------------------

  async listProjects(): Promise<ProjectSummaryDto[]> {
    return this.request<ProjectSummaryDto[]>("/api/v1/projects");
  }

  async getProject(id: string): Promise<ProjectDetailDto> {
    return this.request<ProjectDetailDto>(`/api/v1/projects/${encodeURIComponent(id)}`);
  }

  async createProject(req: CreateProjectRequestDto): Promise<ProjectDetailDto> {
    return this.request<ProjectDetailDto>("/api/v1/projects", "POST", req);
  }

  async saveProject(id: string, req: SaveProjectRequestDto): Promise<ProjectDetailDto> {
    return this.request<ProjectDetailDto>(`/api/v1/projects/${encodeURIComponent(id)}`, "PUT", req);
  }

  async validateProject(id: string): Promise<ProjectValidationResponseDto> {
    return this.request<ProjectValidationResponseDto>(`/api/v1/projects/${encodeURIComponent(id)}/validate`, "POST");
  }

  async loadProjectSim(id: string): Promise<{ status: string; message: string; project_id: string }> {
    return this.request<{ status: string; message: string; project_id: string }>(`/api/v1/projects/${encodeURIComponent(id)}/load-sim`, "POST");
  }

  async getCrossReference(id: string, tagOrEntity: string): Promise<CrossReferenceResultDto> {
    return this.request<CrossReferenceResultDto>(
      `/api/v1/projects/${encodeURIComponent(id)}/cross-reference?tag_or_entity=${encodeURIComponent(tagOrEntity)}`
    );
  }

  async listAdvancedControl(id: string): Promise<AdvancedControlStrategyDto[]> {
    return this.request<AdvancedControlStrategyDto[]>(`/api/v1/projects/${encodeURIComponent(id)}/advanced-control`);
  }

  async saveAdvancedControl(id: string, strategy: AdvancedControlStrategyDto): Promise<AdvancedControlStrategyDto> {
    return this.request<AdvancedControlStrategyDto>(
      `/api/v1/projects/${encodeURIComponent(id)}/advanced-control`,
      "POST",
      strategy
    );
  }

  async deleteAdvancedControl(id: string, strategyId: string): Promise<{ status: string; id: string }> {
    return this.request<{ status: string; id: string }>(
      `/api/v1/projects/${encodeURIComponent(id)}/advanced-control/${encodeURIComponent(strategyId)}`,
      "DELETE"
    );
  }

  async getNetworkDiagnostics(): Promise<NetworkDiagnosticsDto> {
    return this.request<NetworkDiagnosticsDto>("/api/v1/network/diagnostics");
  }

  async listTemplates(category?: string): Promise<TemplateSummaryDto[]> {
    const query = category ? `?category=${encodeURIComponent(category)}` : "";
    return this.request<TemplateSummaryDto[]>(`/api/v1/library/templates${query}`);
  }

  async listDomainPackages(): Promise<DomainPackageSummaryDto[]> {
    return this.request<DomainPackageSummaryDto[]>("/api/v1/library/domains");
  }

  async getDomainPackage(id: string): Promise<DomainPackageDetailDto> {
    return this.request<DomainPackageDetailDto>(`/api/v1/library/domains/${encodeURIComponent(id)}`);
  }

  async controlEquipment(
    req: EquipmentControlRequestDto
  ): Promise<{ status: string; instance_id: string; action: string }> {
    return this.request<{ status: string; instance_id: string; action: string }>(
      "/api/v1/equipment/control",
      "POST",
      req
    );
  }

  async queryHistory(params: {
    run_id: string;
    point_id: string;
    start_time?: number;
    end_time?: number;
    limit?: number;
    bucket_count?: number;
  }): Promise<any> {
    const query = new URLSearchParams();
    query.set("run_id", params.run_id);
    query.set("point_id", params.point_id);
    if (params.start_time !== undefined) query.set("start_time", params.start_time.toString());
    if (params.end_time !== undefined) query.set("end_time", params.end_time.toString());
    if (params.limit !== undefined) query.set("limit", params.limit.toString());
    if (params.bucket_count !== undefined) query.set("bucket_count", params.bucket_count.toString());
    return this.request(`/api/v1/history/query?${query.toString()}`);
  }

  async queryEvents(params: {
    run_id: string;
    start_time?: number;
    end_time?: number;
  }): Promise<any[]> {
    const query = new URLSearchParams();
    query.set("run_id", params.run_id);
    if (params.start_time !== undefined) query.set("start_time", params.start_time.toString());
    if (params.end_time !== undefined) query.set("end_time", params.end_time.toString());
    return this.request<any[]>(`/api/v1/history/events?${query.toString()}`);
  }

  // ── Phase 15: Training Studio API ─────────────────────────────────────

  // Scenario Catalog
  async listScenarios(): Promise<ScenarioSummaryDto[]> {
    return this.request<ScenarioSummaryDto[]>("/api/v1/training/scenarios");
  }

  async getScenario(id: string): Promise<ScenarioAuthoringDto> {
    return this.request<ScenarioAuthoringDto>(`/api/v1/training/scenarios/${id}`);
  }

  async saveScenario(dto: ScenarioAuthoringDto): Promise<ScenarioAuthoringDto> {
    return this.request<ScenarioAuthoringDto>("/api/v1/training/scenarios", "POST", dto);
  }

  async updateScenario(id: string, dto: ScenarioAuthoringDto): Promise<ScenarioAuthoringDto> {
    return this.request<ScenarioAuthoringDto>(`/api/v1/training/scenarios/${id}`, "PUT", dto);
  }

  async deleteScenario(id: string): Promise<void> {
    await this.request<unknown>(`/api/v1/training/scenarios/${id}`, "DELETE" as any);
  }

  async duplicateScenario(id: string, newId: string, newName?: string): Promise<ScenarioAuthoringDto> {
    const q = new URLSearchParams({ new_id: newId });
    if (newName) q.set("new_name", newName);
    return this.request<ScenarioAuthoringDto>(
      `/api/v1/training/scenarios/${id}/duplicate?${q.toString()}`,
      "POST"
    );
  }

  async validateScenario(id: string, projectId?: string): Promise<ScenarioValidationResultDto> {
    const q = projectId ? `?project_id=${projectId}` : "";
    return this.request<ScenarioValidationResultDto>(
      `/api/v1/training/scenarios/${id}/validate${q}`,
      "POST"
    );
  }

  // Training Sessions
  async listSessions(): Promise<TrainingSessionSummaryDto[]> {
    return this.request<TrainingSessionSummaryDto[]>("/api/v1/training/sessions");
  }

  async createSession(req: CreateTrainingSessionRequest): Promise<TrainingSessionDetailDto> {
    return this.request<TrainingSessionDetailDto>("/api/v1/training/sessions", "POST", req);
  }

  async getSessionDetail(id: string): Promise<TrainingSessionDetailDto> {
    return this.request<TrainingSessionDetailDto>(`/api/v1/training/sessions/${id}`);
  }

  async loadSessionProject(id: string, req: LoadSessionProjectRequest): Promise<TrainingSessionDetailDto> {
    return this.request<TrainingSessionDetailDto>(`/api/v1/training/sessions/${id}/load`, "POST", req);
  }

  async prepareSession(id: string): Promise<TrainingSessionDetailDto> {
    return this.request<TrainingSessionDetailDto>(`/api/v1/training/sessions/${id}/prepare`, "POST");
  }

  async startSession(id: string): Promise<TrainingSessionDetailDto> {
    return this.request<TrainingSessionDetailDto>(`/api/v1/training/sessions/${id}/start`, "POST");
  }

  async pauseSession(id: string): Promise<TrainingSessionDetailDto> {
    return this.request<TrainingSessionDetailDto>(`/api/v1/training/sessions/${id}/pause`, "POST");
  }

  async resumeSession(id: string): Promise<TrainingSessionDetailDto> {
    return this.request<TrainingSessionDetailDto>(`/api/v1/training/sessions/${id}/resume`, "POST");
  }

  async resetSession(id: string): Promise<TrainingSessionDetailDto> {
    return this.request<TrainingSessionDetailDto>(`/api/v1/training/sessions/${id}/reset`, "POST");
  }

  async completeSession(id: string): Promise<EvaluationReportDto> {
    return this.request<EvaluationReportDto>(`/api/v1/training/sessions/${id}/complete`, "POST");
  }

  async abortSession(id: string, req: AbortSessionRequest): Promise<TrainingSessionDetailDto> {
    return this.request<TrainingSessionDetailDto>(`/api/v1/training/sessions/${id}/abort`, "POST", req);
  }

  async recordTraineeAction(id: string, req: RecordTraineeActionRequest): Promise<TraineeActionDto> {
    return this.request<TraineeActionDto>(`/api/v1/training/sessions/${id}/action`, "POST", req);
  }

  async getEvaluation(sessionId: string): Promise<EvaluationReportDto> {
    return this.request<EvaluationReportDto>(`/api/v1/training/sessions/${sessionId}/evaluation`);
  }

  async getReplaySummary(sessionId: string): Promise<ReplaySessionSummaryDto> {
    return this.request<ReplaySessionSummaryDto>(`/api/v1/training/sessions/${sessionId}/replay`);
  }

  async getReplayFrame(sessionId: string, simTime: number): Promise<ReplayFrameDto> {
    return this.request<ReplayFrameDto>(
      `/api/v1/training/sessions/${sessionId}/replay/frame?sim_time=${simTime}`
    );
  }

  async getDebrief(sessionId: string): Promise<TrainingDebriefDto> {
    return this.request<TrainingDebriefDto>(`/api/v1/training/sessions/${sessionId}/debrief`);
  }

  async getBranches(sessionId: string): Promise<ScenarioBranchDto[]> {
    return this.request<ScenarioBranchDto[]>(`/api/v1/training/sessions/${sessionId}/branches`);
  }

  async triggerBranch(sessionId: string, branchId: string): Promise<ScenarioBranchDto> {
    return this.request<ScenarioBranchDto>(`/api/v1/training/sessions/${sessionId}/branches/${encodeURIComponent(branchId)}/trigger`, "POST");
  }

  // --- Virtual Controller Runtime APIs ---

  async listControllers(): Promise<ControllerSummaryDto[]> {
    return this.request<ControllerSummaryDto[]>("/api/v1/controllers");
  }

  async getController(id: string): Promise<ControllerDetailDto> {
    return this.request<ControllerDetailDto>(`/api/v1/controllers/${encodeURIComponent(id)}`);
  }

  async updateControllerState(id: string, action: string): Promise<ControllerDetailDto> {
    return this.request<ControllerDetailDto>(
      `/api/v1/controllers/${encodeURIComponent(id)}/state`,
      "POST",
      { action }
    );
  }

  async updateControlModule(
    controllerId: string,
    moduleId: string,
    req: ControlModuleActionRequestDto
  ): Promise<{ status: string }> {
    return this.request<{ status: string }>(
      `/api/v1/controllers/${encodeURIComponent(controllerId)}/modules/${encodeURIComponent(moduleId)}`,
      "POST",
      req
    );
  }

  async injectControllerFault(
    id: string,
    req: ControllerFaultInjectionRequestDto
  ): Promise<{ status: string }> {
    return this.request<{ status: string }>(
      `/api/v1/controllers/${encodeURIComponent(id)}/fault`,
      "POST",
      req
    );
  }

  async triggerRedundancySwitchover(id: string): Promise<{ status: string }> {
    return this.request<{ status: string }>(
      `/api/v1/controllers/${encodeURIComponent(id)}/redundancy/switchover`,
      "POST",
      {}
    );
  }

  // --- I/O Hardware Abstraction Layer (HAL) APIs ---

  async listHalSystems(): Promise<HalSystemSummaryDto[]> {
    return this.request<HalSystemSummaryDto[]>("/api/v1/hal/systems");
  }

  async getHalChannel(address: string): Promise<HalChannelDto> {
    return this.request<HalChannelDto>(`/api/v1/hal/channels/${encodeURIComponent(address)}`);
  }

  async setChannelFault(address: string, req: HalFaultRequestDto): Promise<{ status: string }> {
    return this.request<{ status: string }>(
      `/api/v1/hal/channels/${encodeURIComponent(address)}/fault`,
      "POST",
      req
    );
  }

  async listHalMappings(): Promise<HalMappingDto[]> {
    return this.request<HalMappingDto[]>("/api/v1/hal/mappings");
  }

  // --- IEC 61131-3 Control Logic APIs ---

  async compileIec(req: IecCompileRequestDto): Promise<IecCompileResponseDto> {
    return this.request<IecCompileResponseDto>("/api/v1/controllers/iec/compile", "POST", req);
  }

  // --- Stage 7: Industrial Protocol & Communication APIs ---

  async listProtocolConnections(): Promise<ProtocolConnectionDto[]> {
    return this.request<ProtocolConnectionDto[]>("/api/v1/communications/connections");
  }

  async getProtocolConnection(id: string): Promise<ProtocolConnectionDto> {
    return this.request<ProtocolConnectionDto>(`/api/v1/communications/connections/${encodeURIComponent(id)}`);
  }

  async createProtocolConnection(conn: ProtocolConnectionDto): Promise<ProtocolConnectionDto> {
    return this.request<ProtocolConnectionDto>("/api/v1/communications/connections", "POST", conn);
  }

  async updateProtocolConnection(id: string, conn: ProtocolConnectionDto): Promise<ProtocolConnectionDto> {
    return this.request<ProtocolConnectionDto>(`/api/v1/communications/connections/${encodeURIComponent(id)}`, "PUT", conn);
  }

  async deleteProtocolConnection(id: string): Promise<{ status: string; deleted_id: string }> {
    return this.request<{ status: string; deleted_id: string }>(`/api/v1/communications/connections/${encodeURIComponent(id)}`, "DELETE");
  }

  async testProtocolConnection(id: string, protocol?: string): Promise<ConnectionTestResultDto> {
    const query = protocol ? `?protocol=${encodeURIComponent(protocol)}` : "";
    return this.request<ConnectionTestResultDto>(`/api/v1/communications/connections/${encodeURIComponent(id)}/test${query}`, "POST");
  }

  async listTagMappings(): Promise<TagMappingDto[]> {
    return this.request<TagMappingDto[]>("/api/v1/communications/mappings");
  }

  async getTagMapping(id: string): Promise<TagMappingDto> {
    return this.request<TagMappingDto>(`/api/v1/communications/mappings/${encodeURIComponent(id)}`);
  }

  async createTagMapping(mapping: TagMappingDto): Promise<TagMappingDto> {
    return this.request<TagMappingDto>("/api/v1/communications/mappings", "POST", mapping);
  }

  async updateTagMapping(id: string, mapping: TagMappingDto): Promise<TagMappingDto> {
    return this.request<TagMappingDto>(`/api/v1/communications/mappings/${encodeURIComponent(id)}`, "PUT", mapping);
  }

  async deleteTagMapping(id: string): Promise<{ status: string; deleted_id: string }> {
    return this.request<{ status: string; deleted_id: string }>(`/api/v1/communications/mappings/${encodeURIComponent(id)}`, "DELETE");
  }

  async getProtocolDiagnostics(): Promise<ProtocolDiagnosticsDto[]> {
    return this.request<ProtocolDiagnosticsDto[]>("/api/v1/communications/diagnostics");
  }

  async getIndustrialArchitecture(): Promise<IndustrialZoneDto[]> {
    return this.request<IndustrialZoneDto[]>("/api/v1/communications/architecture");
  }

  async getProjectCommunications(projectId: string): Promise<CommunicationsConfigDto> {
    return this.request<CommunicationsConfigDto>(`/api/v1/projects/${encodeURIComponent(projectId)}/communications`);
  }

  async saveProjectCommunications(projectId: string, cfg: CommunicationsConfigDto): Promise<CommunicationsConfigDto> {
    return this.request<CommunicationsConfigDto>(`/api/v1/projects/${encodeURIComponent(projectId)}/communications`, "PUT", cfg);
  }

  // --- Stage 9: Safety Instrumented System (SIS) APIs ---

  async getSisSummary(): Promise<SisStatusSummaryDto> {
    return this.request<SisStatusSummaryDto>("/api/v1/sis");
  }

  async listSifs(): Promise<SifDetailDto[]> {
    return this.request<SifDetailDto[]>("/api/v1/sis/sifs");
  }

  async getSif(id: string): Promise<SifDetailDto> {
    return this.request<SifDetailDto>(`/api/v1/sis/sifs/${encodeURIComponent(id)}`);
  }

  async resetSif(id: string, actor: string): Promise<{ status: string; message: string }> {
    return this.request<{ status: string; message: string }>(
      `/api/v1/sis/sifs/${encodeURIComponent(id)}/reset`,
      "POST",
      { actor }
    );
  }

  async setSifBypass(
    id: string,
    req: SifBypassRequestDto
  ): Promise<{ status: string; sif_id: string; active: boolean }> {
    return this.request<{ status: string; sif_id: string; active: boolean }>(
      `/api/v1/sis/sifs/${encodeURIComponent(id)}/bypass`,
      "POST",
      req
    );
  }

  async runProofTest(
    id: string,
    technician: string
  ): Promise<{ status: string; sif_id: string; message: string }> {
    return this.request<{ status: string; sif_id: string; message: string }>(
      `/api/v1/sis/sifs/${encodeURIComponent(id)}/proof-test`,
      "POST",
      { technician }
    );
  }

  async getCauseEffectMatrix(): Promise<CauseEffectMatrixDto> {
    return this.request<CauseEffectMatrixDto>("/api/v1/sis/cause-effect");
  }

  // ── Stage 13: Scientific Validation & Benchmarks ─────────────────────────

  async getValidationSummary(): Promise<ValidationExecutiveSummaryDto> {
    return this.request<ValidationExecutiveSummaryDto>("/api/v1/validation/summary");
  }

  async listValidationCases(domain?: string): Promise<ValidationCaseSummaryDto[]> {
    const q = domain ? `?domain=${encodeURIComponent(domain)}` : "";
    return this.request<ValidationCaseSummaryDto[]>(`/api/v1/validation/cases${q}`);
  }

  async getValidationCase(id: string): Promise<ValidationCaseDetailDto> {
    return this.request<ValidationCaseDetailDto>(`/api/v1/validation/cases/${encodeURIComponent(id)}`);
  }

  async runValidationBenchmarks(req?: ValidationRunRequestDto): Promise<ValidationRunResponseDto> {
    return this.request<ValidationRunResponseDto>("/api/v1/validation/run", "POST", req || {});
  }

  async listValidationReferences(): Promise<ReferenceDatasetDto[]> {
    return this.request<ReferenceDatasetDto[]>("/api/v1/validation/references");
  }

  async listValidationModels(): Promise<ModelValidationSummaryDto[]> {
    return this.request<ModelValidationSummaryDto[]>("/api/v1/validation/models");
  }

  async getValidationManifest(): Promise<ValidationManifestDto> {
    return this.request<ValidationManifestDto>("/api/v1/validation/manifest");
  }

  // ── Stage 14: Enterprise, HA, Cluster, Audit & Observability ───────────

  async getClusterTopology(): Promise<ClusterTopologyDto> {
    return this.request<ClusterTopologyDto>("/api/v1/cluster/topology");
  }

  async triggerFailover(req: FailoverRequestDto): Promise<PromotionResultDto> {
    return this.request<PromotionResultDto>("/api/v1/cluster/failover", "POST", req);
  }

  async injectClusterFailure(req: FailureInjectionRequestDto): Promise<FailureInjectionStatusDto> {
    return this.request<FailureInjectionStatusDto>("/api/v1/cluster/inject-failure", "POST", req);
  }

  async clearClusterFailure(): Promise<FailureInjectionStatusDto> {
    return this.request<FailureInjectionStatusDto>("/api/v1/cluster/clear-failure", "POST");
  }

  async getClusterFailureStatus(): Promise<FailureInjectionStatusDto> {
    return this.request<FailureInjectionStatusDto>("/api/v1/cluster/failure-status");
  }

  async getReplicationSnapshot(): Promise<ReplicationSnapshotDto> {
    return this.request<ReplicationSnapshotDto>("/api/v1/cluster/replication/snapshot", "POST");
  }

  async applyReplicationSnapshot(snap: ReplicationSnapshotDto): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/cluster/replication/apply", "POST", snap);
  }

  async getAuditEvents(params?: { actor?: string; role?: string; action?: string; source?: string; limit?: number }): Promise<AuditEventDto[]> {
    const q = new URLSearchParams();
    if (params?.actor) q.set("actor", params.actor);
    if (params?.role) q.set("role", params.role);
    if (params?.action) q.set("action", params.action);
    if (params?.source) q.set("source", params.source);
    if (params?.limit) q.set("limit", String(params.limit));
    const qs = q.toString() ? `?${q.toString()}` : "";
    return this.request<AuditEventDto[]>(`/api/v1/audit/events${qs}`);
  }

  async getAuditSummary(): Promise<AuditSummaryDto> {
    return this.request<AuditSummaryDto>("/api/v1/audit/summary");
  }

  async recordAuditEvent(ev: AuditEventDto): Promise<{ status: string }> {
    return this.request<{ status: string }>("/api/v1/audit/record", "POST", ev);
  }

  async getSubsystemObservability(): Promise<SubsystemObservabilityDto> {
    return this.request<SubsystemObservabilityDto>("/api/v1/observability/telemetry");
  }

  async getDiagnostics(params?: { domain?: string; severity?: string }): Promise<DiagnosticItemDto[]> {
    const q = new URLSearchParams();
    if (params?.domain) q.set("domain", params.domain);
    if (params?.severity) q.set("severity", params.severity);
    const qs = q.toString() ? `?${q.toString()}` : "";
    return this.request<DiagnosticItemDto[]>(`/api/v1/observability/diagnostics${qs}`);
  }

  async getDiagnosticsSummary(): Promise<DiagnosticsSummaryDto> {
    return this.request<DiagnosticsSummaryDto>("/api/v1/observability/diagnostics/summary");
  }

  async validateBackup(bundle: unknown): Promise<RestoreValidationDto> {
    return this.request<RestoreValidationDto>("/api/v1/admin/backup/validate", "POST", bundle);
  }

  async restoreBackup(bundle: unknown): Promise<RestoreResultDto> {
    return this.request<RestoreResultDto>("/api/v1/admin/restore", "POST", bundle);
  }

  async createBackupBundle(): Promise<unknown> {
    return this.request<unknown>("/api/v1/admin/backup", "POST");
  }

  // ==========================================================================
  // Stage 15: Industrial Cyber Security & IEC 62443 Methods
  // ==========================================================================

  async getSecurityStatus(): Promise<SecurityStatusDto> {
    return this.request<SecurityStatusDto>("/api/v1/security/status");
  }

  async getSecurityZones(): Promise<PurdueZoneDto[]> {
    return this.request<PurdueZoneDto[]>("/api/v1/security/zones");
  }

  async getSecurityConduits(): Promise<PurdueConduitDto[]> {
    return this.request<PurdueConduitDto[]>("/api/v1/security/conduits");
  }

  async updateConduitPolicy(
    req: UpdateConduitPolicyRequestDto
  ): Promise<PurdueConduitDto> {
    return this.request<PurdueConduitDto>(
      "/api/v1/security/conduits/policy",
      "POST",
      req
    );
  }

  async getSecurityIncidents(limit = 50): Promise<SecurityIncidentDto[]> {
    return this.request<SecurityIncidentDto[]>(
      `/api/v1/security/incidents?limit=${limit}`
    );
  }

  async verifyAuditIntegrity(): Promise<AuditVerificationResultDto> {
    return this.request<AuditVerificationResultDto>(
      "/api/v1/security/audit/verify",
      "POST"
    );
  }
}

export const sharedApiClient = new ApiClient();
export const client = sharedApiClient;
authStore.subscribe((_, token) => {
  sharedApiClient.setAuthToken(token);
});

