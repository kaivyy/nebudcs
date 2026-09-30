/**
 * NebuDCS Protocol Types & Data Transfer Objects.
 * Authoritative mirror of `crates/nebudcs-protocol`.
 */

export const PROTOCOL_VERSION = "1.0.0";

export type TagQualityDto = "GOOD" | "UNCERTAIN" | "BAD" | "NOT_CONNECTED";

export type TagValueDto = number | boolean;

export interface TagValueSampleDto {
  tag: string;
  value: TagValueDto;
  quality: TagQualityDto;
  unit?: string;
}

export type AlarmStateDto =
  | "NORMAL"
  | "ACTIVE_UNACK"
  | "ACTIVE_ACK"
  | "CLEARED"
  | "SHELVED";

export type AlarmSeverityDto =
  | "DIAGNOSTIC"
  | "ADVISORY"
  | "WARNING"
  | "CRITICAL";

export interface AlarmSnapshotDto {
  alarm_name: string;
  state: AlarmStateDto;
  severity: AlarmSeverityDto;
  pv: number;
  trip_limit: number;
}

export interface StateSnapshotDto {
  snapshot_id: string;
  sim_time_sec: number;
  tick_count: number;
  tags: TagValueSampleDto[];
  active_alarms: AlarmSnapshotDto[];
}

export interface DeltaUpdateDto {
  seq: number;
  sim_time_sec: number;
  tick_count: number;
  changed_tags: TagValueSampleDto[];
}

export interface ServerHealthDto {
  status: string;
  simulation_running: boolean;
  sim_time_sec: number;
  tick_count: number;
  speed_multiplier: number;
  connected_clients: number;
  protocol_version: string;
  historian_healthy: boolean;
}

export interface HistoricalEventDto {
  event_id: string;
  event_type: string;
  sim_time_sec: number;
  tick_count: number;
  source_id: string;
  description: string;
  payload: Record<string, unknown>;
  real_timestamp_ms: number;
}

export interface HistoricalSampleDto {
  point_id: number;
  sim_time_sec: number;
  tick_count: number;
  value: number;
  quality: TagQualityDto;
  real_timestamp_ms: number;
}

export type PidModeDto = "MANUAL" | "AUTO" | "CASCADE" | "REMOTE_CASCADE";

export type SimControlAction =
  | { action: "play" }
  | { action: "pause" }
  | { action: "step" }
  | { action: "set_speed_multiplier"; multiplier: number }
  | { action: "reset" };

export type RuntimeCommand =
  | ({ type: "sim_control" } & SimControlAction)
  | { type: "write_tag"; tag: string; value: TagValueDto }
  | { type: "set_setpoint"; loop_name: string; sp: number }
  | { type: "set_mode"; loop_name: string; mode: PidModeDto }
  | { type: "set_manual_output"; loop_name: string; output: number }
  | { type: "acknowledge_alarm"; alarm_name: string }
  | { type: "shelve_alarm"; alarm_name: string; duration_seconds: number }
  | { type: "reset_protection"; block_name: string }
  | {
      type: "equipment";
      action: "start" | "stop" | "reset_trip" | "set_var";
      instance_id: string;
      variable_name?: string;
      value?: number;
    };

export interface EquipmentControlRequestDto {
  instance_id: string;
  action: "start" | "stop" | "reset_trip" | "set_var";
  variable_name?: string;
  value?: number;
}

export type FaceplateKind =
  | "control_loop"
  | "pump"
  | "valve"
  | "compressor"
  | "vessel"
  | "heat_exchanger"
  | "instrument"
  | "reactor"
  | "separator"
  | "column"
  | "turbine"
  | "tank"
  | "furnace"
  | "equipment";

export interface FaceplateTarget {
  kind: FaceplateKind;
  id: string;
  name: string;
  description?: string;
  pvTag?: string;
  spTag?: string;
  outTag?: string;
  mode?: PidModeDto;
  sp?: number;
  unit?: string;
  rangeMin?: number;
  rangeMax?: number;
  equipmentId?: string;
  parameters?: Record<string, number>;
}

export interface ClientCapabilities {
  supports_binary: boolean;
  max_batch_size: number;
}

export type ClientMessage =
  | {
      type: "handshake";
      protocol_version: string;
      client_id: string;
      capabilities: ClientCapabilities;
    }
  | {
      type: "subscribe";
      tags: string[];
      all_tags: boolean;
      include_alarms: boolean;
    }
  | {
      type: "unsubscribe";
      tags: string[];
    }
  | {
      type: "request_snapshot";
    }
  | {
      type: "command";
      command_id: string;
      command: RuntimeCommand;
    }
  | {
      type: "ping";
      nonce: number;
    };

export type ServerMessage =
  | {
      type: "handshake_ack";
      payload: {
        protocol_version: string;
        server_version: string;
        session_id: string;
        model_version: string;
      };
    }
  | {
      type: "snapshot";
      payload: StateSnapshotDto;
    }
  | {
      type: "delta";
      payload: DeltaUpdateDto;
    }
  | {
      type: "alarm_update";
      payload: AlarmSnapshotDto[];
    }
  | {
      type: "event";
      payload: HistoricalEventDto;
    }
  | {
      type: "command_ack";
      payload: {
        command_id: string;
        success: boolean;
        message?: string;
        error_code?: string;
      };
    }
  | {
      type: "pong";
      payload: {
        nonce: number;
      };
    }
  | {
      type: "error";
      payload: {
        code: string;
        message: string;
        details?: string;
        correlation_id?: string;
      };
    }
  | {
      type: "scenario_update";
      payload: ScenarioStatusDto;
    };

export interface ActiveFaultDto {
  id: string;
  target: string;
  category: string;
  elapsed_sec: number;
  duration_sec?: number;
}

export interface ScenarioStatusDto {
  scenario_id?: string;
  scenario_name?: string;
  state: string;
  elapsed_sec: number;
  duration_limit_sec?: number;
  active_faults: ActiveFaultDto[];
  active_disturbances_count: number;
  events_total: number;
  events_fired: number;
  score: number;
  mandatory_completed: boolean;
}

export interface TemplatePortDto {
  id: string;
  name: string;
  domain: string;
  direction: string;
}

export interface TemplateSummaryDto {
  id: string;
  name: string;
  version: string;
  category: string;
  source: string;
  description: string;
  parameter_count: number;
  port_count: number;
  curve_count: number;
  trip_count: number;
  author: string;
  ports?: TemplatePortDto[];
}

export interface TemplateDetailDto {
  id: string;
  name: string;
  version: string;
  category: string;
  source: string;
  description: string;
  author: string;
  tags: string[];
  icon_hint?: string;
  definition_json: string;
}

export interface PackageSummaryDto {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  sub_component_count: number;
  internal_stream_count: number;
  external_port_count: number;
  dependency_count: number;
}

export interface PackageDetailDto {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  package_json: string;
}

export interface ValidatePackageResponseDto {
  valid: boolean;
  package_id: string;
  resolved_dependencies: string[];
  errors: string[];
  warnings: string[];
}

export interface ImportLibraryItemRequestDto {
  content: string;
  format?: string;
}

export interface InstantiateTemplateRequestDto {
  template_id: string;
  version?: string;
  instance_id?: string;
  name: string;
  parameter_overrides?: Record<string, number>;
}

export interface InstantiatePackageRequestDto {
  package_id: string;
  version?: string;
  instance_prefix: string;
  parameter_overrides?: Record<string, number>;
}

export interface ExportLibraryItemResponseDto {
  id: string;
  version: string;
  format: string;
  content: string;
}

export function isCompatibleProtocol(version: string): boolean {
  const parts = version.split(".");
  if (parts.length !== 3) return false;
  return parts[0] === "1" && parts[1] === "0";
}

// ---------------------------------------------------------------------------
// Phase 13 Engineering Studio & Project Management DTOs
// ---------------------------------------------------------------------------

export interface Position2dDto {
  x: number;
  y: number;
}

export interface ProjectSummaryDto {
  id: string;
  name: string;
  description: string;
  author: string;
  schema_version: string;
  plant_count: number;
  equipment_count: number;
  stream_count: number;
  created: string;
  modified: string;
}

export interface StudioEquipmentDto {
  id: string;
  definition_id: string;
  definition_version: string;
  name: string;
  parameters: Record<string, number>;
  position?: Position2dDto;
  display_width?: number;
  display_height?: number;
  rotation?: number; // 0, 90, 180, 270
  scale?: number;
}

export interface StudioStreamDto {
  id: string;
  name: string;
  stream_type: string;
  source_equipment_id: string;
  source_port_id: string;
  sink_equipment_id: string;
  sink_port_id: string;
  composition: Record<string, number>;
  route_points: Position2dDto[];
  diameter?: number;
  length?: number;
  roughness?: number;
  elevation_change?: number;
  loss_k?: number;
}

export interface NodeDiagnosticDto {
  id: string;
  name: string;
  pressure_pa: number;
  pressure_bar: number;
  temperature_c: number;
  density_kg_m3: number;
  enthalpy_j_kg: number;
  phase: string;
  elevation_m: number;
}

export interface EdgeDiagnosticDto {
  id: string;
  name: string;
  from_node: string;
  to_node: string;
  mass_flow_kg_s: number;
  volumetric_flow_m3_h: number;
  pressure_drop_kpa: number;
  velocity_m_s: number;
  reynolds: number;
  friction_factor: number;
  flow_regime: string;
}

export interface NetworkDiagnosticsDto {
  converged: boolean;
  iterations: number;
  max_residual: number;
  residual_norm: number;
  nodes: NodeDiagnosticDto[];
  edges: EdgeDiagnosticDto[];
  offending_component?: string;
  error_message?: string;
}


export interface StudioInstrumentDto {
  id: string;
  tag: string;
  equipment_id?: string;
  port_id?: string;
  measurement_kind: string;
  range_min: number;
  range_max: number;
  unit: string;
  position?: Position2dDto;
}

export interface StudioControlLoopDto {
  id: string;
  name: string;
  pv_tag: string;
  sp: number;
  out_tag: string;
  kp: number;
  ti: number;
  td: number;
  reverse_acting: boolean;
  mode: string;
  output_min: number;
  output_max: number;
  position?: Position2dDto;
}

export interface StudioAlarmDto {
  id: string;
  name: string;
  pv_tag: string;
  alarm_type: string;
  severity: string;
  trip_limit: number;
  deadband: number;
  description: string;
}

export interface StudioInterlockDto {
  id: string;
  name: string;
  source_tag: string;
  condition: string;
  threshold: number;
  target_action: string;
  target_equipment?: string;
  latched: boolean;
  description: string;
}

export type DisplayTypeDto =
  | "plant_overview"
  | "area_overview"
  | "system_overview"
  | "unit_process_graphic"
  | "subsystem_graphic"
  | "group_display"
  | "point_detail"
  | "trend"
  | "alarms"
  | "diagnostics";

export interface ViewportConfigDto {
  x: number;
  y: number;
  width: number;
  height: number;
  zoom: number;
}

export interface DisplayDto {
  id: string;
  name: string;
  display_type: DisplayTypeDto;
  area_id?: string;
  system_id?: string;
  subsystem_id?: string;
  unit_id?: string;
  equipment_ids: string[];
  instrument_ids: string[];
  stream_ids: string[];
  viewport?: ViewportConfigDto;
  is_default?: boolean;
  order?: number;
  description?: string;
  placements?: DisplayElementPlacementDto[];
}

export interface DisplayElementPlacementDto {
  id?: string;
  display_id?: string;
  equipment_id: string;
  x: number;
  y: number;
  width?: number;
  height?: number;
  rotation?: number;
  scale?: number;
  created_at?: string;
  updated_at?: string;
}

export interface PlantUnitDto {
  id: string;
  name: string;
  subsystem_id?: string;
  system_id?: string;
  area_id?: string;
  equipment_ids: string[];
  display_ids: string[];
  description?: string;
}

export interface PlantSubsystemDto {
  id: string;
  name: string;
  system_id?: string;
  units: PlantUnitDto[];
  description?: string;
}

export interface PlantSystemDto {
  id: string;
  name: string;
  area_id?: string;
  subsystems: PlantSubsystemDto[];
  default_display_id?: string;
  description?: string;
}

export interface PlantAreaDto {
  id: string;
  name: string;
  systems: PlantSystemDto[];
  description?: string;
}

export interface PlantStructureDto {
  plant_name: string;
  areas: PlantAreaDto[];
  displays: DisplayDto[];
}

export interface NavigationLinkDto {
  id: string;
  source_display_id: string;
  source_element_id?: string;
  target_display_id: string;
  target_element_id?: string;
  label: string;
  behavior?: "focus_and_center" | "open_faceplate" | "switch_display_only" | string;
  description?: string;
}

export interface ProjectDetailDto {
  id: string;
  name: string;
  schema_version: string;
  description: string;
  author: string;
  created: string;
  modified: string;
  equipment: StudioEquipmentDto[];
  streams: StudioStreamDto[];
  instruments: StudioInstrumentDto[];
  control_loops: StudioControlLoopDto[];
  alarms: StudioAlarmDto[];
  interlocks: StudioInterlockDto[];
  controllers?: ControllerConfigDto[];
  io_systems?: HalSystemSummaryDto[];
  io_mappings?: HalMappingDto[];
  tags?: ProjectTagDto[];
  advanced_control?: AdvancedControlStrategyDto[];
  communications?: CommunicationsConfigDto;
  sis?: SisConfigDto;
  plant_structure?: PlantStructureDto;
  areas?: PlantAreaDto[];
  displays?: DisplayDto[];
  navigation_links?: NavigationLinkDto[];
  boundary_conditions?: BoundaryConditionDto[];
  initial_conditions?: InitialConditionDto[];
}

export interface BoundaryConditionDto {
  id: string;
  name: string;
  condition_type: "feed" | "utility" | "sink" | string;
  target_equipment_id?: string;
  target_port?: string;
  target_stream_id?: string;
  flow_value: number;
  flow_unit: string;
  pressure_value: number;
  pressure_unit: string;
  temperature_value: number;
  temperature_unit: string;
  phase: string;
  density?: number;
  molecular_weight?: number;
  enthalpy?: number;
  quality?: number;
  composition?: Record<string, number>;
  validity_min_flow?: number;
  validity_max_flow?: number;
  source_provenance?: string;
  enabled: boolean;
}

export interface InitialConditionDto {
  id: string;
  entity_id: string;
  variable: string;
  value: number;
  unit?: string;
  description?: string;
}

export interface AdvancedControlStrategyDto {
  id: string;
  name: string;
  strategy_type: "Cascade" | "Ratio" | "FeedForward" | "Override" | "SplitRange" | "AntiSurge" | "ValveCharacterization" | string;
  description: string;
  enabled: boolean;
  parameters: Record<string, string>;
}

export interface ValidationResultDto {
  severity: "error" | "warning" | "info" | "engineering_review" | string;
  code: string;
  message: string;
  entity_id?: string;
}

export interface ProjectValidationResponseDto {
  valid: boolean;
  error_count: number;
  warning_count: number;
  info_count: number;
  review_count: number;
  results: ValidationResultDto[];
}

export interface CreateProjectRequestDto {
  name: string;
  description?: string;
  author?: string;
}

export interface SaveProjectRequestDto {
  project: ProjectDetailDto;
}

// ── Phase 15: Training Studio DTOs ────────────────────────────────────────

export type SessionLifecycleState =
  | "CREATED"
  | "PREPARING"
  | "READY"
  | "LOADED"
  | "RUNNING"
  | "PAUSED"
  | "COMPLETED"
  | "EVALUATED"
  | "ABORTED"
  | "FAULTED";

export type OperatorErrorClassificationDto =
  | "Correct"
  | "Incorrect"
  | "Forbidden"
  | "Late"
  | "Missing"
  | "PartiallyCorrect"
  | "Inefficient";

export type SafetyTripPolicyDto =
  | "Expected"
  | "Avoidable"
  | "Acceptable"
  | "Failure";

export interface TrainingDebriefDto {
  process_performance: string;
  operator_performance: string;
  safety_assessment: string;
  outcome_verdict: string;
  recommendations: string[];
}

export interface ScenarioBranchDto {
  branch_id: string;
  target_step_id: string;
  description: string;
  condition_summary: string;
  triggered: boolean;
}

export interface TrainingSessionSummaryDto {
  id: string;
  name: string;
  project_id: string;
  scenario_id: string;
  instructor_id: string;
  trainee_id: string;
  state: SessionLifecycleState;
  current_sim_time_sec: number;
  created_at_ms: number;
}

export interface TrainingSessionDetailDto {
  id: string;
  name: string;
  description: string;
  project_id: string;
  scenario_id: string;
  instructor_id: string;
  trainee_id: string;
  state: SessionLifecycleState;
  scenario_version?: string;
  runtime_snapshot?: string;
  score?: number;
  completion_state?: string;
  created_at_ms: number;
  started_at_ms?: number;
  completed_at_ms?: number;
  simulation_start_time_sec: number;
  current_simulation_time_sec: number;
  abort_reason?: string;
  metadata: Record<string, string>;
}

export interface CreateTrainingSessionRequest {
  name: string;
  description?: string;
  project_id?: string;
  scenario_id?: string;
  instructor_id: string;
  trainee_id: string;
}

export interface LoadSessionProjectRequest {
  project_id: string;
  scenario_id: string;
}

export interface AbortSessionRequest {
  reason: string;
}

export interface TraineeActionDto {
  action_id: string;
  session_id: string;
  sim_time_sec: number;
  wall_clock_ms: number;
  actor: string;
  target_entity: string;
  action_type: string;
  requested_value?: number;
  status: string;
  details?: string;
  error_classification?: OperatorErrorClassificationDto;
}

export interface RecordTraineeActionRequest {
  session_id: string;
  actor: string;
  target_entity: string;
  action_type: string;
  requested_value?: number;
  command_payload?: unknown;
}

export interface CriterionResultDto {
  criterion_id: string;
  title: string;
  passed: boolean;
  score_awarded: number;
  score_possible: number;
  penalty_deducted: number;
  explanation: string;
  evidence: Record<string, string>;
  error_classification?: OperatorErrorClassificationDto;
  safety_trip_assessment?: string;
}

export interface EvaluationReportDto {
  session_id: string;
  scenario_id: string;
  trainee_id: string;
  total_score: number;
  max_possible_score: number;
  percentage: number;
  passed: boolean;
  results: CriterionResultDto[];
  evaluated_at_ms: number;
  debrief?: TrainingDebriefDto;
}

export interface ReplayMarkerDto {
  marker_id: string;
  sim_time_sec: number;
  kind: string;
  source_id: string;
  label: string;
  details: string;
  severity: number;
}

export interface ReplayFrameDto {
  sim_time_sec: number;
  tags: Record<string, number>;
  active_alarms: string[];
  active_trips: string[];
  recent_actions: TraineeActionDto[];
}

export interface ReplaySessionSummaryDto {
  session_id: string;
  project_id: string;
  scenario_id: string;
  start_sim_time_sec: number;
  end_sim_time_sec: number;
  duration_sec: number;
  markers: ReplayMarkerDto[];
}

export interface ScenarioSummaryDto {
  id: string;
  name: string;
  description: string;
  author: string;
  version: string;
  compatible_project_id?: string;
  duration_limit_sec?: number;
  event_count: number;
  objective_count: number;
}

export interface ScenarioAuthoringDto {
  id: string;
  name: string;
  description: string;
  author: string;
  version: string;
  compatible_project_id?: string;
  duration_limit_sec?: number;
  random_seed: number;
  initial_conditions_json: string;
  timeline_json: string;
  objectives_json: string;
  criteria_json: string;
}

export interface ScenarioValidationResultDto {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

export interface OperatingRangeDto {
  quantity: string;
  min_value: number;
  max_value: number;
  unit: string;
  normal_operating_value: number;
}

export interface ScientificReferenceDto {
  citation_key: string;
  title: string;
  authors: string[];
  publication_year: number;
  source: string;
  doi_or_url?: string;
  governing_concept: string;
  validity_domain: string;
}

export interface ModelClassificationDto {
  subsystem: string;
  classification: "Theoretical" | "StandardsBased" | "LiteratureBased" | "Empirical" | "Approximation" | string;
  governing_equation: string;
  primary_assumptions: string[];
  conservation_verification: string;
}

export interface DomainPackageSummaryDto {
  id: string;
  name: string;
  version: string;
  domain_kind: string;
  description: string;
  author: string;
  strategy_count: number;
  scenario_count: number;
  template_count: number;
  scientific_references?: ScientificReferenceDto[];
  model_classifications?: ModelClassificationDto[];
}

export interface DomainPackageDetailDto {
  id: string;
  name: string;
  version: string;
  domain_kind: string;
  description: string;
  author: string;
  thermodynamic_requirements: string[];
  utility_requirements: string[];
  composition_defaults: Record<string, number>;
  operating_ranges: Record<string, OperatingRangeDto>;
  control_strategies: string[];
  scenario_presets: string[];
  equipment_templates: string[];
  plant_template_ids: string[];
  validation_rules: string[];
  scientific_references?: ScientificReferenceDto[];
  model_classifications?: ModelClassificationDto[];
}

export interface ControllerSummaryDto {
  id: string;
  name: string;
  lifecycle: string;
  task_count: number;
  scan_count: number;
  avg_scan_duration_us: number;
  max_scan_duration_us: number;
  watchdog_trips: number;
  overrun_count: number;
  is_faulted: boolean;
}

export interface ControllerTaskDto {
  id: string;
  name: string;
  task_type: string;
  period_seconds?: number;
  priority: number;
  enabled: boolean;
  execution_count: number;
  overrun_count: number;
  last_duration_us: number;
  max_duration_us: number;
  program_count: number;
}

export interface ControllerFaultDto {
  code: string;
  message: string;
  timestamp_sim: number;
  recoverable: boolean;
}

export interface ControllerDetailDto {
  id: string;
  name: string;
  description: string;
  lifecycle: string;
  watchdog_timeout_seconds: number;
  tasks: ControllerTaskDto[];
  input_points: string[];
  output_points: string[];
  scan_count: number;
  task_execution_count: number;
  avg_scan_duration_us: number;
  max_scan_duration_us: number;
  last_scan_duration_us: number;
  watchdog_trips: number;
  overrun_count: number;
  fault?: ControllerFaultDto;
  cpu_profile?: CpuProfileDto;
  memory_profile?: MemoryProfileDto;
  firmware_profile?: FirmwareProfileDto;
  redundancy?: RedundancyStatusDto;
  control_modules?: DcsControlModuleDto[];
}

export interface UpdateControllerStateRequestDto {
  action: string;
}

export interface HalDiagnosticsDto {
  channels_scanned: number;
  faulted_channels: number;
  last_scan_duration_us: number;
  max_scan_duration_us: number;
  avg_scan_duration_us: number;
}

export interface HalChannelDto {
  id: string;
  channel_number: number;
  address: string;
  name: string;
  kind: "AI" | "AO" | "DI" | "DO" | string;
  value: number;
  raw_counts: number;
  quality: string;
  fault_state: string;
  status: string;
  engineering_unit: string;
  lrv: number;
  urv: number;
  fallback_behavior?: string;
  forced_value?: number;
}

export interface HalModuleDto {
  slot: number;
  name: string;
  module_type: string;
  status: string;
  channels: HalChannelDto[];
}

export interface HalChassisDto {
  chassis_number: number;
  name: string;
  bus_ok: boolean;
  modules: HalModuleDto[];
}

export interface HalRackDto {
  rack_number: number;
  name: string;
  power_ok: boolean;
  chassis: HalChassisDto[];
}

export interface HalMappingDto {
  controller_id: string;
  signal_key: string;
  channel_address: string;
  direction: "Input" | "Output" | string;
  target_tag?: string;
  target_actuator?: string;
  priority?: number;
}

export interface HalSystemSummaryDto {
  id: string;
  name: string;
  rack_count: number;
  module_count: number;
  channel_count: number;
  mapping_count: number;
  diagnostics: HalDiagnosticsDto;
  racks: HalRackDto[];
  mappings: HalMappingDto[];
}

export interface HalFaultRequestDto {
  fault?: string;
  forced_value?: number;
}

// --- IEC 61131-3 Control Logic Runtime Types ---

export interface IecVariableDto {
  name: string;
  data_type: string;
  value: string;
  scope: string;
  address_binding?: string;
}

export interface IecCompileRequestDto {
  language: "ST" | "LD" | "FBD" | "SFC" | string;
  source_text: string;
}

export interface IecCompileResponseDto {
  success: boolean;
  error_message?: string;
  symbol_count: number;
  statement_count: number;
  variables: IecVariableDto[];
}

export interface IecRoutineDto {
  name: string;
  language: "ST" | "LD" | "FBD" | "SFC" | string;
  source_text: string;
  enabled: boolean;
  statement_count: number;
  symbol_count: number;
}

export interface ProgramConfigDto {
  id: string;
  name: string;
  description?: string;
  program_type: string;
  routines: IecRoutineDto[];
}

export interface TaskConfigDto {
  id: string;
  name: string;
  task_type: "periodic" | "event" | "continuous" | string;
  period_seconds: number;
  priority: number;
  watchdog_seconds: number;
  enabled: boolean;
  programs: ProgramConfigDto[];
}

export interface ControllerConfigDto {
  id: string;
  name: string;
  description?: string;
  watchdog_timeout_seconds: number;
  enabled: boolean;
  tasks: TaskConfigDto[];
  cpu_profile?: CpuProfileDto;
  memory_profile?: MemoryProfileDto;
  firmware_profile?: FirmwareProfileDto;
  redundancy?: RedundancyConfigDto;
  control_modules?: DcsControlModuleDto[];
}

export interface ProjectTagDto {
  id: string;
  name: string;
  description?: string;
  data_type: string;
  engineering_unit: string;
  source: string;
  direction: string;
  controller_id?: string;
  channel_address?: string;
  process_entity?: string;
  alarm_id?: string;
}

export interface CrossReferenceQueryDto {
  tag_or_entity: string;
}

export interface CrossReferenceEntryDto {
  tag: string;
  entity_type: string;
  role: string;
  location: string;
  details: string;
}

export interface CrossReferenceResultDto {
  target: string;
  entries: CrossReferenceEntryDto[];
}

// =========================================================================
// Stage 7: Industrial Protocol & Communication Architecture DTOs
// =========================================================================

export interface ProtocolConnectionDto {
  id: string;
  name: string;
  protocol: string;
  endpoint: string;
  enabled: boolean;
  state: string;
  zone_id: string;
  timeout_ms: number;
  reconnect_delay_ms: number;
  use_tls: boolean;
  active_tags: number;
  last_comm_ms: number;
}

export interface TagMappingDto {
  id: string;
  connection_id: string;
  external_address: string;
  canonical_tag: string;
  direction: string;
  datatype: string;
  scale: number;
  offset: number;
  engineering_unit?: string;
  deadband: number;
  update_rate_ms: number;
  stale_timeout_ms: number;
  access: string;
  current_value?: string;
  quality: string;
}

export interface ProtocolDiagnosticsDto {
  connection_id: string;
  protocol: string;
  state: string;
  is_healthy: boolean;
  messages_sent: number;
  messages_received: number;
  bytes_sent: number;
  bytes_received: number;
  read_operations: number;
  write_operations: number;
  error_count: number;
  reconnect_count: number;
  active_subscriptions: number;
  stale_tags_count: number;
  queue_depth: number;
  last_latency_ms: number;
  last_comm_ms: number;
}

export interface IndustrialZoneDto {
  id: string;
  name: string;
  level: string;
  permitted_protocols: string[];
  trust_boundary: boolean;
  security_policy: string;
}

export interface CommunicationsConfigDto {
  connections: ProtocolConnectionDto[];
  tag_mappings: TagMappingDto[];
  zones: IndustrialZoneDto[];
}

export interface ConnectionTestResultDto {
  connection_id: string;
  endpoint: string;
  protocol: string;
  success: boolean;
  latency_ms: number;
  message: string;
}

// =========================================================================
// Stage 8: Vendor-Neutral Virtual PLC/DCS Controller Architecture DTOs
// =========================================================================

export interface CpuProfileDto {
  execution_budget_us: number;
  max_scan_frequency_hz: number;
  utilization_threshold_pct: number;
  architecture: string;
}

export interface MemoryProfileDto {
  program_memory_bytes: number;
  data_memory_bytes: number;
  retentive_memory_bytes: number;
  program_memory_used_bytes?: number;
  data_memory_used_bytes?: number;
  retentive_memory_used_bytes?: number;
}

export interface FirmwareProfileDto {
  version: string;
  build: string;
  iec_standard: string;
}

export interface RedundancyConfigDto {
  role: string;
  partner_controller_id?: string;
  sync_frequency_hz: number;
  switchover_timeout_seconds: number;
}

export interface RedundancyStatusDto {
  role: string;
  status: string;
  partner_id?: string;
  sync_frequency_hz: number;
  switchover_timeout_seconds: number;
  last_sync_sim_time: number;
  sync_cycle_count: number;
}

export interface DcsControlModuleDto {
  id: string;
  name: string;
  description?: string;
  mode: "MAN" | "AUTO" | "CAS" | "RCAS" | string;
  execution_rate_seconds: number;
  pv_signal: string;
  sp: number;
  out_signal: string;
  strategy_type: "PID" | "Cascade" | "Ratio" | "Override" | "Manual" | string;
  kp: number;
  ti: number;
  td: number;
  output_min: number;
  output_max: number;
  permissives: string[];
  interlocks: string[];
  safe_position: number;
  current_pv: number;
  current_out: number;
  is_interlocked: boolean;
  permissives_ok: boolean;
  tracking_active: boolean;
}

export interface ControlModuleActionRequestDto {
  mode?: string;
  sp?: number;
  manual_out?: number;
}

export interface ControllerFaultInjectionRequestDto {
  fault_type: string;
  description?: string;
  recoverable?: boolean;
}

// ── Stage 9: Safety Instrumented System (SIS) DTOs ─────────────────────────

export interface SifConfigDto {
  id: string;
  name: string;
  description: string;
  hazard_id?: string;
  hazard_scenario?: string;
  target_sil?: string;
  direction: "High" | "Low" | string;
  threshold: number;
  reset_deadband: number;
  trip_delay_seconds: number;
  architecture: "1oo1" | "1oo2" | "2oo2" | "2oo3" | "2oo4" | string;
  degradation_policy: "DegradeTo1oo2" | "DegradeTo2oo2" | "DegradeTo1oo1" | "TripOnFault" | "FreezeState" | string;
  sensor_tags: string[];
  final_elements: SifFinalElementConfigDto[];
  latched: boolean;
  reset_permissives: string[];
  pfd_avg_estimate?: number;
}

export interface SifFinalElementConfigDto {
  target_equipment: string;
  actuator_var: string;
  safe_state: "ValveClosed" | "ValveOpen" | "DriverTrip" | string;
  de_energize_to_trip: boolean;
  action_type: string;
  feedback_tag?: string;
}

export interface SisConfigDto {
  enabled: boolean;
  sifs: SifConfigDto[];
}

export interface SisStatusSummaryDto {
  enabled: boolean;
  sif_count: number;
  armed_count: number;
  tripped_count: number;
  bypassed_count: number;
  degraded_count: number;
  proof_tests_count: number;
  active_overrides_count: number;
}

export interface SifDetailDto {
  id: string;
  name: string;
  description: string;
  hazard_id?: string;
  hazard_scenario?: string;
  target_sil?: string;
  direction: string;
  threshold: number;
  reset_deadband: number;
  trip_delay_seconds: number;
  architecture: string;
  degradation_policy: string;
  state: "Armed" | "DelayTiming" | "Tripped" | "Bypassed" | "Degraded" | "InProofTest" | string;
  trip_latched: boolean;
  sensor_channels: SifSensorChannelDto[];
  final_elements: SifFinalElementDto[];
  diagnostics: SifDiagnosticsDto;
  bypass?: SifBypassDto;
  pfd_avg_estimate?: number;
}

export interface SifSensorChannelDto {
  id: string;
  tag: string;
  value: number;
  quality: string;
  unit?: string;
  faulted: boolean;
}

export interface SifFinalElementDto {
  target_equipment: string;
  actuator_var: string;
  safe_state: string;
  de_energize_to_trip: boolean;
  action_type: string;
  feedback_tag?: string;
}

export interface SifDiagnosticsDto {
  total_sensors: number;
  healthy_sensors: number;
  faulted_sensors: number;
  final_elements_count: number;
  trip_count: number;
  last_trip_sim_time?: number;
  last_trip_cause?: string;
  last_reset_sim_time?: number;
  voting_scheme: string;
  delay_elapsed_seconds: number;
}

export interface SifBypassDto {
  bypass_type: string;
  authorized_by: string;
  reason: string;
  start_sim_time: number;
  duration_seconds?: number;
  active: boolean;
}

export interface SifBypassRequestDto {
  bypass_type: string;
  authorized_by: string;
  reason: string;
  duration_seconds?: number;
  active: boolean;
}

export interface SifResetRequestDto {
  actor: string;
}

export interface ProofTestExecutionRequestDto {
  technician: string;
}

export interface CauseEffectMatrixDto {
  causes: CauseEntryDto[];
  effects: EffectEntryDto[];
  cells: CauseEffectCellDto[];
}

export interface CauseEntryDto {
  sif_id: string;
  description: string;
  hazard_id?: string;
  threshold: number;
  voting: string;
  status: string;
}

export interface EffectEntryDto {
  equipment_id: string;
  variable: string;
  safe_state: string;
  action_type: string;
}

export interface CauseEffectCellDto {
  cause_index: number;
  effect_index: number;
  active_link: boolean;
  commanded_state?: string;
  is_asserted: boolean;
}

export type ValidityStateDto =
  | "validated"
  | "within_validity_range"
  | "extrapolated"
  | "empirical_correlation"
  | "theoretical"
  | "approximation"
  | "property_unavailable"
  | "bad_quality"
  | "stale"
  | "communication_lost";

export interface AlarmFloodMetricsDto {
  alarm_rate_per_10min: number;
  is_flood: boolean;
  flood_threshold_per_10min: number;
  total_active: number;
  unacknowledged: number;
  acknowledged: number;
  shelved: number;
  suppressed: number;
  standing_count: number;
  stale_count: number;
  first_out_alarm?: string;
  first_out_timestamp?: number;
}

export interface AlarmSummaryDto {
  metrics: AlarmFloodMetricsDto;
  active_alarms: AlarmSnapshotDto[];
  shelved_alarms: string[];
}

export interface EquipmentDiagnosticsDto {
  equipment_id: string;
  equipment_name: string;
  equipment_type: string;
  validity_state: ValidityStateDto;
  state_label: string;
  speed_rpm?: number;
  flow_m3_h?: number;
  suction_pressure_kpa?: number;
  discharge_pressure_kpa?: number;
  head_m?: number;
  power_kw?: number;
  efficiency_pct?: number;
  npsha_m?: number;
  npshr_m?: number;
  cavitation_margin_m?: number;
  cavitation_regime?: string;
  vibration_mm_s?: number;
  surge_margin_pct?: number;
  pressure_ratio?: number;
  compressor_operating_state?: string;
  inlet_temp_c?: number;
  outlet_temp_c?: number;
  thermal_duty_kw?: number;
  delta_t_c?: number;
  lmtd_c?: number;
  valve_command_pct?: number;
  valve_actual_pct?: number;
  valve_fail_action?: string;
  is_tripped: boolean;
  active_alarms: string[];
  active_interlocks: string[];
  sis_override_active: boolean;
}

export interface FirstOutEventDto {
  run_id: string;
  first_out_alarm?: string;
  initiating_event_type?: string;
  timestamp_sim?: number;
  disturbance_chain: HistoricalEventDto[];
}

export interface OperatorActionAuditDto {
  timestamp_sim: number;
  wall_clock_ms: number;
  actor: string;
  role: string;
  action: string;
  target: string;
  previous_state?: string;
  requested_state: string;
  authorized: boolean;
  rejection_reason?: string;
}

// ── Stage 13: Scientific Validation, Benchmark & Certification DTOs ─────────

export type EvidenceClassificationDto =
  | "PEER_REVIEWED_EXPERIMENTAL"
  | "PUBLISHED_BENCHMARK"
  | "PUBLIC_DATASET"
  | "ENGINEERING_REFERENCE"
  | "STANDARD_REFERENCE"
  | "MANUFACTURER_DATA"
  | "ANALYTICAL_REFERENCE"
  | "ENGINEERING_CONSISTENCY"
  | "SYNTHETIC_TEST";

export type ValidationStatusDto =
  | "NOT_VALIDATED"
  | "ENGINEERING_CONSISTENT"
  | "BENCHMARKED"
  | "VALIDATED"
  | "VALIDATED_WITH_LIMITATIONS"
  | "EXTRAPOLATED"
  | "INVALID"
  | "INSUFFICIENT_DATA";

export type ToleranceThresholdProvenanceDto =
  | "SOURCE_DEFINED"
  | "ENGINEERING_DEFINED"
  | "NUMERICAL"
  | "PROJECT_DEFINED";

export interface ScientificCitationDto {
  title: string;
  authors_or_organization: string;
  publication_year: number;
  doi_or_identifier?: string;
  standard_id?: string;
  url?: string;
}

export interface ApplicabilityEnvelopeDto {
  min_temperature_k: number;
  max_temperature_k: number;
  min_pressure_pa: number;
  max_pressure_pa: number;
  supported_phases: string[];
  supported_fluids: string[];
  limitation_notes?: string;
}

export interface ErrorMetricsDto {
  mean_absolute_error: number;
  root_mean_squared_error: number;
  max_absolute_deviation: number;
  max_relative_error_pct: number;
  mean_bias_error: number;
  conservation_residual: number;
}

export interface ValidationCaseSummaryDto {
  id: string;
  name: string;
  domain: string;
  tested_feature: string;
  classification: EvidenceClassificationDto;
  status: ValidationStatusDto;
  standard_id?: string;
  passed: boolean;
  max_relative_error_pct: number;
  relative_tolerance_pct: number;
  last_executed_iso: string;
}

export interface ValidationCaseDetailDto {
  id: string;
  name: string;
  domain: string;
  tested_feature: string;
  model_name: string;
  classification: EvidenceClassificationDto;
  status: ValidationStatusDto;
  tolerance_provenance: ToleranceThresholdProvenanceDto;
  citation: ScientificCitationDto;
  applicability_envelope: ApplicabilityEnvelopeDto;
  relative_tolerance_pct: number;
  absolute_tolerance: number;
  metrics: ErrorMetricsDto;
  passed: boolean;
  last_executed_iso: string;
  execution_duration_ms: number;
  notes: string;
  disclaimer: string;
}

export interface ReferenceDatasetDto {
  id: string;
  name: string;
  domain: string;
  source_citation: string;
  standard_organization?: string;
  point_count: number;
  parameters: string[];
}

export interface ModelValidationSummaryDto {
  model_name: string;
  domain: string;
  status: ValidationStatusDto;
  cases_count: number;
  cases_passed: number;
  worst_case_relative_error_pct: number;
  validity_summary: string;
}

export interface ValidationExecutiveSummaryDto {
  total_cases: number;
  passed_cases: number;
  failed_cases: number;
  pass_rate_pct: number;
  overall_status: string;
  domain_counts: [string, number, number][]; // [domain, total, passed]
  classification_counts: [string, number][]; // [classification, count]
  timestamp_iso: string;
  engine_version: string;
  compliance_disclaimer: string;
}

export interface ValidationRunRequestDto {
  domain_filter?: string;
  case_ids?: string[];
}

export interface ValidationRunResponseDto {
  executed_cases: number;
  passed_cases: number;
  failed_cases: number;
  total_duration_ms: number;
  cases: ValidationCaseSummaryDto[];
  summary: ValidationExecutiveSummaryDto;
}

export interface ValidationManifestDto {
  version: string;
  generated_at_iso: string;
  system_id: string;
  summary: ValidationExecutiveSummaryDto;
  cases: ValidationCaseDetailDto[];
  models: ModelValidationSummaryDto[];
  references: ReferenceDatasetDto[];
  disclaimers: string[];
}

// ==========================================
// Stage 14: Enterprise & HA Protocol DTOs
// ==========================================

export type NodeRoleDto =
  | "control_simulation"
  | "standby"
  | "historian"
  | "communication"
  | "web_gateway";

export type HaStateDto =
  | "STANDALONE"
  | "STARTING"
  | "PRIMARY"
  | "STANDBY"
  | "SYNCHRONIZING"
  | "SYNCHRONIZED"
  | "DEGRADED"
  | "FAILOVER"
  | "FAILED"
  | "RECOVERING"
  | "SHUTTING_DOWN";

export interface ClusterNodeDto {
  node_id: string;
  name: string;
  role: NodeRoleDto;
  ha_state: HaStateDto;
  endpoint_url: string;
  is_local: boolean;
  epoch: number;
  last_heartbeat_secs_ago: number;
  health_status: string;
  replication_lag_ms: number;
}

export interface ClusterTopologyDto {
  cluster_id: string;
  current_leader_id?: string;
  leader_epoch: number;
  lease_remaining_secs: number;
  local_node_id: string;
  local_ha_state: HaStateDto;
  nodes: ClusterNodeDto[];
  split_brain_fencing_active: boolean;
  total_nodes_count: number;
  active_nodes_count: number;
  replication_health: string;
}

export interface HaHeartbeatMessage {
  sender_node_id: string;
  sender_role: NodeRoleDto;
  sender_ha_state: HaStateDto;
  epoch: number;
  sequence: number;
  sim_time_secs: number;
  sim_tick: number;
  timestamp_iso: string;
}

export interface ReplicationSnapshotDto {
  epoch: number;
  sequence: number;
  sim_time_secs: number;
  sim_tick: number;
  sim_state: string;
  project_version: string;
  tag_count: number;
  active_alarms_count: number;
  sis_tripped: boolean;
  historian_seq: number;
  checksum_sha256: string;
  payload_json: string;
}

export interface FailoverRequestDto {
  target_leader_id: string;
  target_epoch: number;
  force: boolean;
  reason: string;
}

export interface PromotionResultDto {
  success: boolean;
  promoted_node_id: string;
  new_epoch: number;
  previous_leader_id?: string;
  message: string;
}

export interface FailureInjectionRequestDto {
  failure_type: string;
  target_node_id?: string;
  duration_secs: number;
  induced_lag_ms?: number;
}

export interface FailureInjectionStatusDto {
  active: boolean;
  failure_type?: string;
  target_node_id?: string;
  remaining_secs: number;
  message: string;
}

export interface AuditEventDto {
  event_id: string;
  timestamp_iso: string;
  sim_time_secs: number;
  actor: string;
  role: string;
  source: string;
  target: string;
  action: string;
  prev_value?: string;
  new_value?: string;
  status: string;
  reason?: string;
  correlation_id?: string;
  previous_hash?: string;
  event_hash?: string;
}

export interface AuditSummaryDto {
  total_events_recorded: number;
  current_buffer_size: number;
  unique_actors_count: number;
  unique_actions_count: number;
  latest_event_timestamp?: string;
}

export interface SubsystemObservabilityDto {
  system_cpu_pct: number;
  system_memory_rss_bytes: number;
  system_uptime_secs: number;
  sim_freq_hz: number;
  sim_dt: number;
  sim_tick: number;
  sim_convergence_failures: number;
  sim_is_deterministic: boolean;
  control_tasks_active: number;
  control_overruns_count: number;
  control_faults_active: number;
  io_channels_total: number;
  io_bad_quality_count: number;
  io_stale_data_count: number;
  protocol_connections_active: number;
  protocol_packet_rate: number;
  protocol_error_count: number;
  sis_active_trips: number;
  sis_bypasses_active: number;
  sis_proof_test_active: boolean;
  historian_write_rate_sps: number;
  historian_queue_depth: number;
  historian_total_samples: number;
  ots_active_sessions: number;
  ots_scenarios_running: number;
  validation_cases_passed: number;
  validation_cases_total: number;
  validation_coverage_pct: number;
  cluster_ha_state: HaStateDto;
  cluster_leader_id?: string;
  cluster_epoch: number;
  cluster_replication_lag_ms: number;
}

export interface DiagnosticItemDto {
  id: string;
  domain: string;
  severity: string;
  title: string;
  description: string;
  entity_id?: string;
  entity_type?: string;
  timestamp_iso: string;
  suggested_action: string;
}

export interface DiagnosticsSummaryDto {
  total_active: number;
  critical_count: number;
  error_count: number;
  warning_count: number;
  info_count: number;
}

export interface RestoreValidationDto {
  valid: boolean;
  bundle_version: string;
  manifest_items_verified: number;
  projects_count: number;
  scenarios_count: number;
  training_count: number;
  has_cluster_config: boolean;
  has_validation_evidence: boolean;
  validation_messages: string[];
}

export interface RestoreResultDto {
  projects_restored: number;
  scenarios_restored: number;
  training_restored: number;
  details: string[];
}

// ============================================================================
// Stage 15: IEC 62443 Industrial Cybersecurity & Purdue Architecture DTOs
// ============================================================================

export type SecurityLevel =
  | "SL1_COINCIDENTAL"
  | "SL2_SIMPLE_MEANS"
  | "SL3_SOPHISTICATED"
  | "SL4_EXTENDED";

export interface PurdueZoneDto {
  zone_id: string;
  name: string;
  level_name: string;
  purdue_level: number;
  target_security_level: string;
  achieved_security_level: string;
  assets_count: number;
  status: string;
  trust_boundary: boolean;
  description: string;
  assigned_roles: string[];
}

export interface PurdueConduitDto {
  conduit_id: string;
  name: string;
  source_zone_id: string;
  destination_zone_id: string;
  allowed_protocols: string[];
  security_level_required: string;
  is_encrypted: boolean;
  packets_inspected: number;
  violations_blocked: number;
  status: string;
  policy_description: string;
}

export interface SecurityIncidentDto {
  incident_id: string;
  timestamp_iso: string;
  severity: string;
  category: string;
  source_ip: string;
  actor: string;
  description: string;
  mitigation_action: string;
  status: string;
}

export interface AuditVerificationResultDto {
  is_valid: boolean;
  total_events_checked: number;
  root_hash: string;
  latest_hash: string;
  tampering_detected: boolean;
  first_invalid_event_id?: string;
  verification_timestamp_iso: string;
  verification_status: string;
}

export interface SecurityStatusDto {
  system_security_level: string;
  active_threat_level: string;
  total_incidents: number;
  active_incidents: number;
  audit_chain_valid: boolean;
  audit_events_count: number;
  root_hash: string;
  mtls_enabled: boolean;
  rbac_enforced: boolean;
  zones_count: number;
  conduits_count: number;
  last_audit_verification_iso?: string;
}

export interface UpdateConduitPolicyRequestDto {
  conduit_id: string;
  allowed_protocols: string[];
  is_encrypted: boolean;
  status: string;
}
