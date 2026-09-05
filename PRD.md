# NebuDCS

## Generic Process Simulation Platform & DCS/PLC Training Environment

---

## 1. Product Vision

Build NebuDCS as a lightweight, extensible, high-performance process simulation and DCS/PLC training platform.

NebuDCS must **NOT** be hardcoded around NHT, TDHT, AH Unibon, Platforming, CCR, CDU, RFCC, or any single refinery configuration.

The core product is a generic simulation engine, equipment framework, process topology system, control system, HMI framework, historian, scenario engine, and engineering designer.

Refinery units are reusable templates/packages built on top of the generic engine.

Users must be able to:

- Create a refinery project from an empty canvas.
- Add and configure equipment.
- Connect process streams.
- Configure utilities.
- Create instruments and tags.
- Configure PID loops.
- Configure interlocks, permissives, trips, ESD and sequences.
- Build P&ID/process graphics.
- Build operator HMI screens.
- Run dynamic simulations.
- Change operating conditions.
- Inject equipment/instrument failures.
- Observe process propagation.
- Record alarms, events and historical values.
- Replay scenarios.
- Save and reopen complete plant designs.
- Create completely custom refinery/unit designs.

The resulting experience should resemble a professional industrial DCS engineering/operator environment while remaining an open, generic simulation platform.

---

## 2. Core Architectural Principle

The simulation engine is the source of truth.

> Visual graphics must never be the source of process state.

Architecture:

```
Rust Core
→ Plant Model
→ Equipment Models
→ Process/Physics Engine
→ Control Engine
→ Alarm/Interlock Engine
→ Historian
→ Scenario Engine
→ API/WebSocket
→ UI
```

The engine must be usable without a graphical interface.

Provide:

- **Desktop Engineering Mode**
- **Web Operator/HMI Mode**
- **Headless CLI Simulation Mode**

All modes must use the same plant model and simulation core.

---

## 3. Technology Direction

### Core

- Rust
- Stable Rust
- Async only where appropriate
- Strong typing
- No unnecessary runtime dependencies
- Deterministic simulation where possible
- Modular crate architecture
- Clear ownership boundaries

### Desktop

- Rust-native
- `egui` and `wgpu` or equivalent lightweight GPU-capable rendering architecture
- No Electron
- No embedded Chromium requirement for the engineering application

### Web

- TypeScript
- Prefer Web Components/lightweight framework-free architecture
- SVG for interactive process graphics
- Canvas/WebGL where large scenes or high-frequency rendering require it
- WebSocket for realtime state
- REST/HTTP for configuration/project operations

> Do NOT introduce React unless profiling and architectural evidence demonstrates a real requirement.

### Persistence

- SQLite for local projects, configuration and historian
- PostgreSQL as an optional future server deployment target
- Project format must be portable and versioned

---

## 4. Repository Architecture

Recommended structure:

```
crates/
  nebudcs-core
  nebudcs-model
  nebudcs-process
  nebudcs-physics
  nebudcs-equipment
  nebudcs-control
  nebudcs-plc
  nebudcs-sfc
  nebudcs-alarm
  nebudcs-historian
  nebudcs-scenario
  nebudcs-scripting
  nebudcs-render
  nebudcs-protocol
  nebudcs-storage

apps/
  nebudcs-desktop
  nebudcs-server
  nebudcs-cli
  nebudcs-web
```

Keep domain logic independent from UI.

---

## 5. Plant Model

Plant hierarchy:

```
Project
→ Plant
→ Area
→ Unit
→ Equipment
→ Instruments
→ Control Loops
→ Streams
→ Utilities
→ Alarms
→ Scenarios
```

The topology must be **graph-based**.

Entities include:

- Equipment nodes
- Stream edges
- Instrument relationships
- Control relationships
- Utility relationships

The system must support arbitrary topology rather than predefined unit layouts.

---

## 6. Equipment Framework

Equipment must be template-based and configurable.

Every equipment type should expose, conceptually:

- identity
- parameters
- ports
- process state
- tags
- alarms
- controls
- failure modes
- validation
- dynamic calculation
- visual definition

Do not create a separate hardcoded class for every refinery-specific equipment arrangement.

Use composable templates and configuration.

### Examples

**Rotating Equipment**

- Pump
- Main Pump
- Auxiliary Pump
- Multistage Pump
- Compressor
- 2-stage Compressor
- 3-stage Compressor
- H2 Compressor
- NASH Compressor
- Blower
- Fan
- Motor
- Steam Turbine
- Turbine-driven Pump

**Heat Transfer**

- Heat Exchanger
- Fin Fan Cooler
- Water Cooler
- Air Cooler
- Condenser
- Reboiler
- Furnace

**Process Equipment**

- Reactor
- Fixed-bed Reactor
- Reactor Train
- CCR Reactor
- Regenerator
- Vessel
- Separator
- Drum
- Tank
- Column
- Fractionator
- Stripper
- Absorber

**Valves**

- Block Valve
- Control Valve
- Check Valve
- ESD Valve
- PSV
- Damper

**Utilities**

- Hydrogen
- Steam HP/MP/LP
- Cooling Water
- Instrument Air
- Plant Air
- Nitrogen
- Fuel Gas
- Electricity
- Flare

---

## 7. Equipment Parameters

Equipment must have configurable engineering parameters.

### Pump

- rated flow
- rated head
- efficiency
- speed
- pump curve
- suction pressure
- discharge pressure
- NPSH
- power
- vibration
- bearing temperature
- operating state

### Compressor

- stage count
- speed
- suction pressure
- discharge pressure
- flow
- temperature
- pressure ratio
- power
- efficiency
- compressor map
- surge margin
- recycle
- anti-surge control
- vibration
- bearing temperature

### Turbine

- inlet pressure
- inlet temperature
- exhaust pressure
- steam flow
- speed
- load
- power
- efficiency
- vibration
- bearing temperature
- lubrication system
- trip state

### Heat Exchanger

- hot/cold inlet/outlet
- flow
- pressure
- pressure drop
- duty
- UA
- fouling
- thermal effectiveness

### Fin Fan

- fan speed
- air flow
- process flow
- inlet/outlet temperature
- ambient temperature
- fan status
- vibration
- motor current

### Vessel

- orientation (vertical / horizontal)
- geometry
- volume
- level
- pressure
- temperature
- inlet ports
- outlet ports
- internals (trays, packing, demister, distributor, baffle)
- PSV
- instrumentation

---

## 8. Reactor Framework

Reactors must support configurable ports and internal models.

Do not assume every reactor has the same inlet/outlet arrangement.

Support:

- top inlet
- bottom inlet
- top outlet
- bottom outlet
- side ports
- catalyst ports where applicable
- gas ports where applicable
- utility ports

### CCR Reactor Trains

Must be composable:

```
R-1
→ Interheater/Furnace
→ R-2
→ Interheater/Furnace
→ R-3
```

### Catalyst Circulation

Must be modeled as a separate network:

```
Spent Catalyst
→ Regenerator
→ Regenerated Catalyst
→ Reactor Train
→ Spent Catalyst
```

### Regenerator Zones

Must support configurable zones. Do not hardcode A-H as universal.

If a project requires zones A, B, C, D, E, F, G, H — they must be created as configuration.

Each zone may expose:

- temperature
- pressure
- gas composition
- flow
- catalyst state
- O2
- CO
- CO2

---

## 9. Furnace Framework

Furnace must be an actual equipment model, not only an SVG.

Support:

- radiant section
- convection section
- tubes
- passes
- burners
- combustion air
- fuel gas
- stack
- damper
- draft
- O2
- process inlet/outlet
- tube skin temperature
- process temperature
- furnace state
- trip conditions

Support multiple furnace instances (F-1, F-2, F-3) without creating separate source-code implementations.

---

## 10. Process Streams

`MaterialStream` is a fundamental domain object.

Support:

- flow
- temperature
- pressure
- composition
- phase
- density
- enthalpy
- quality
- molecular weight where applicable

Composition must be configurable.

Hydrogen networks must NOT be represented as a single boolean or fixed H2 property.

Example:

```
Platforming
→ H2 Separator
→ H2 Compressor
→ H2 Header
→ Recycle
→ Export
→ Other Units
```

Branching networks must be supported.

---

## 11. Utilities

Utilities are first-class networks.

Support:

- hydrogen
- steam
- cooling water
- instrument air
- plant air
- nitrogen
- fuel gas
- electricity
- flare

Utility degradation must propagate into connected equipment.

Examples:

- Instrument air pressure decrease → control valve response degradation → process response
- Cooling water reduction → exchanger duty reduction → temperature changes
- Electrical failure → motor/pump/compressor state changes

---

## 12. Control System

Provide generic DCS/PLC-style control functionality.

Support:

- PID
- manual mode
- automatic mode
- cascade
- remote setpoint
- tracking
- output limits
- anti-windup
- permissives
- interlocks
- trips
- ESD
- latches
- reset
- sequence logic
- timers
- state machines
- SFC
- ladder-style logic where practical

All control logic operates through the common tag system.

---

## 13. Tag System

Tags must be globally addressable.

Examples:

```
K101.SPEED
K101.FLOW
K101.PDIS
K101.PSUC

K101.Z001A
K101.Z001B
K101.Z001C

FCV101.PV
FCV101.SP
FCV101.OUT
```

Tags must support:

- type
- engineering unit
- value
- quality
- timestamp
- source
- writable/read-only state
- alarm metadata

---

## 14. Dynamic Equipment Behavior

Equipment behavior must be physically or empirically configurable.

Do not use simplistic universal linear relationships.

### Example: Compressor

```
Speed
→ flow/pressure relationship
→ power
→ efficiency
→ vibration
→ surge margin
```

Support compressor maps and configurable vibration behavior:

- Normal operating range → Warning range → Critical range → Trip range

Allow nonlinear behavior and critical-speed regions.

---

## 15. Turbine + Auxiliary Systems

Turbine-driven equipment must support auxiliary dependencies.

Support:

- lubrication
- seal system
- bearing temperature
- bearing vibration
- oil pressure
- oil temperature
- auxiliary pump
- trip logic

### Failure Propagation Example

```
Lube oil pressure decrease
→ bearing condition degradation
→ vibration increase
→ alarm
→ trip
→ driven equipment shutdown
→ process propagation
```

---

## 16. Alarm System

Support alarm types:

- HI / HIHI
- LO / LOLO
- deviation
- rate of change
- bad quality
- communication failure
- equipment failure
- trip
- advisory / warning / critical

Alarm states:

- normal
- active
- acknowledged
- cleared
- shelved (where implemented)

Provide alarm priorities and configurable limits.

---

## 17. Historian

Record:

- timestamp
- tag
- value
- quality
- alarm
- event
- operator action

Support:

- realtime trend
- historical trend
- multiple tag overlay
- time range selection
- replay
- scenario playback

Use aggregation/downsampling for long time ranges.

---

## 18. Fault Injection

Provide configurable failure modes:

- pump failure
- compressor trip / overspeed
- valve stuck open / stuck closed / slow response
- transmitter frozen / drift / bias / noisy / signal loss
- furnace trip / flame failure
- cooling water loss
- instrument air loss
- hydrogen loss
- electrical failure
- utility degradation
- bearing degradation
- vibration increase

Failures must propagate through the process model.

---

## 19. Scenario Engine

Users can create scenarios:

```
Scenario
→ initial conditions
→ timed events
→ faults
→ operator actions
→ expected responses
→ grading/assessment where applicable
```

### Example: Compressor Speed Increase

```
RPM increase
→ flow change
→ pressure change
→ power increase
→ vibration response
→ alarm
→ potential trip
→ downstream process changes
```

> The simulator must never simply display predetermined outcomes. Outcomes must come from the plant model.

---

## 20. HMI Architecture

HMI must be separate from simulation logic.

Provide:

- plant overview
- unit overview
- process graphic
- equipment graphic
- faceplate
- alarm summary
- alarm detail
- trend
- event list
- navigation
- operator controls

Support HMI style inspired by industrial DCS/HMI conventions (HMI 505-style visual organization where appropriate), but do not hardcode the engine to a specific vendor.

---

## 21. Faceplates

Every controllable equipment type should provide a reusable faceplate.

### Pump Faceplate

- status / start / stop
- speed / flow / pressure / power
- alarms

### Control Valve Faceplate

- PV / SP / output
- AUTO/MAN / limits / alarms

### Compressor Faceplate

- RPM / flow / suction pressure / discharge pressure / temperature / power
- surge margin
- vibration points
- bearing temperatures
- start/stop / alarms / trip status

---

## 22. P&ID Editor

Build an engineering canvas.

Support:

- drag/drop equipment
- equipment library
- port connections
- streams
- instruments
- control loops
- labels / tags
- zoom / pan
- selection / multi-selection
- snapping / alignment
- copy/paste / undo/redo
- grouping / hierarchy

The P&ID must be generated from the plant model. Equipment symbols must remain linked to actual equipment instances.

---

## 23. Three Equipment Representations

Each equipment should have:

1. **Engineering P&ID symbol**
2. **Operator process graphic**
3. **Faceplate**

All three reference the same equipment model and tags.

---

## 24. Unit Templates

Build reusable unit templates using generic equipment.

Initial templates:

- NHT
- TDHT
- AH Unibon
- Platforming
- CCR
- CDU
- RFCC

Templates must remain editable after insertion. Users must be able to:

- add/remove equipment
- reconnect streams
- modify parameters, controls, utilities, instruments
- modify HMI graphics

Do not implement these units as hardcoded simulation branches.

---

## 25. RFCC Template

Provide a configurable RFCC process package composed of generic equipment.

Potential components:

- feed system
- riser
- reactor
- stripper
- regenerator
- catalyst circulation
- main fractionator
- wet gas compressor
- gas plant
- air blower
- flue gas system
- cyclones
- utilities / controls / alarms / flare connections

Exact topology must remain configurable.

---

## 26. Custom Refinery Designer

Provide:

```
New Project
→ Empty Plant
→ Add Units
→ Add Equipment
→ Connect Streams
→ Configure Utilities
→ Configure Controls
→ Configure HMI
→ Simulate
```

Users must be able to create completely custom refinery architectures, including non-standard layouts for experimentation and training.

Example:

```
CDU → NHT → CCR → RFCC → Gas Plant
→ H2 Network → Steam Network → Cooling Water → Flare
```

---

## 27. Engineering Validation

Provide design validation.

| Severity | Example |
|---|---|
| ERROR | Control valve has no valid upstream/downstream process connection. |
| ERROR | Invalid stream connection / Incompatible process phases. |
| WARNING | Vessel has no configured relief protection. |
| WARNING | Pump discharge has no downstream connection. |
| WARNING | Compressor recycle configured without recycle control. |
| INFO | Advisory / informational notices. |
| ENGINEERING REVIEW | Unusual configurations requiring review. |

Do not automatically reject unusual configurations unless the engine truly cannot simulate them.

---

## 28. UI Strategy

### Desktop Engineering Station

**Purpose:** project creation, P&ID editing, equipment/control configuration, SFC/logic editing, plant design, scenario authoring, offline engineering.

**Technology:** Rust + egui/wgpu or equivalent lightweight native rendering.

### Web Operator/Training Station

**Purpose:** HMI, operator controls, alarms, trends, historian, scenarios, remote simulation access.

**Technology:** TypeScript + Web Components, SVG, Canvas/WebGL when required, WebSocket.

Both applications must use the same Rust simulation core.

---

## 29. Performance Architecture

Separate update rates:

| Layer | Rate |
|---|---|
| Simulation | ~50-100 Hz where required |
| Control | ~10-20 Hz or equipment-specific |
| Historian | Configurable sampling |
| WebSocket | Batched state updates |
| Rendering | ~60 FPS where possible |

Techniques:

- delta updates
- batching
- dirty state tracking
- efficient serialization
- binary protocol where justified
- downsampling for historian data
- worker threads where useful

Avoid unnecessary allocations in the simulation loop.

---

## 30. Headless CLI

Provide `nebudcs-cli`:

- load project
- run simulation
- set initial conditions
- execute scenarios
- inject faults
- export historian
- run deterministic replay
- run validation
- execute automated tests

Essential for CI and regression testing.

---

## 31. Scripting

Provide optional user scripting without requiring Rust recompilation.

Use a sandboxed embedded scripting language such as **Rhai** where appropriate.

Scripts can define:

- custom equipment behavior
- custom calculations
- scenario logic
- training events
- derived tags

Scripts must not automatically receive unrestricted filesystem, network, or process access.

---

## 32. Project Format

Projects must be portable and contain:

- plant topology
- equipment instances + parameters
- streams / instruments / tags
- controls / alarms
- HMI layout
- scenarios
- historian configuration
- metadata + version information

Provide schema versioning and migration support.

---

## 33. Testing

Every equipment model must have automated tests.

Required categories:

- unit tests
- equipment tests
- control tests
- integration tests
- plant topology tests
- scenario tests
- deterministic replay tests
- serialization tests
- performance tests
- regression tests

### Example: Compressor Tests

- startup / shutdown
- speed change
- pressure response / flow response
- anti-surge
- surge condition
- vibration response
- trip / restart

---

## 34. Development Roadmap

| Phase | Focus |
|---|---|
| Phase 0 | Architecture and repository foundation |
| Phase 1 | Simulation clock, plant state, tags, topology, events |
| Phase 2 | Generic equipment framework: stream, pump, valve, control valve, vessel, heat exchanger, motor |
| Phase 3 | Rotating equipment: compressor, multistage, H2, NASH, turbine, blower, fan, vibration model |
| Phase 4 | Process equipment: reactor, reactor train, regenerator, furnace, column, separator, drum, tank, fin fan, cooler |
| Phase 5 | Control engine: PID, AUTO/MAN, cascade, permissive, interlock, trip, ESD, sequence, SFC |
| Phase 6 | Alarm + historian |
| Phase 7 | Web HMI |
| Phase 8 | P&ID engineering editor |
| Phase 9 | Desktop Engineering Station |
| Phase 10 | Utilities and flare |
| Phase 11 | Unit templates: NHT, TDHT, AH Unibon, Platforming, CCR, CDU, RFCC |
| Phase 12 | Scenario/fault engine |
| Phase 13 | Custom refinery designer |
| Phase 14 | Engineering validation |
| Phase 15 | Advanced physics and higher-fidelity models |

---

## 35. UI Development Strategy

Do not postpone UI until the engine is finished. Develop **vertical slices**.

| Slice | Components |
|---|---|
| First | Pump → Simulation → Tag → WebSocket → HMI → Faceplate → Alarm → Historian |
| Second | Compressor → RPM → Flow → Pressure → Vibration → Alarm → Trip |
| Third | Furnace → Temperature → Fuel → Draft → Damper → Trip |
| Fourth | Reactor → temperature → pressure → flow → reaction/process response |
| Fifth | Complete small unit → P&ID → control → HMI → historian → fault scenario |

Only expand equipment libraries after the vertical slice architecture is proven.

---

## 36. Safety and Engineering Boundary

NebuDCS is a simulation/training/engineering sandbox.

> Do not represent simulated behavior as validated live-plant operating guidance.

Real refinery configurations, control logic, trip limits, equipment curves, alarm thresholds, and process parameters must be based on authorized engineering documentation and validated data when modeling an actual facility.

The platform must not assume that a generic template exactly represents a particular refinery.

---

## 37. Definition of Done

The project is considered architecturally successful when a user can:

1. Create an empty refinery.
2. Drag a pump onto the canvas.
3. Add a vessel.
4. Add a heat exchanger.
5. Connect process streams.
6. Add a control valve.
7. Create tags.
8. Configure a PID.
9. Run the simulation.
10. Change pump speed.
11. Observe flow/pressure changes.
12. Observe downstream effects.
13. Open a faceplate.
14. See alarms.
15. View trends.
16. Inject a pump/compressor/valve fault.
17. Observe fault propagation.
18. Save the project.
19. Reopen the project.
20. Replay the scenario.
21. Add a second unit.
22. Connect utility networks.
23. Build a complete custom process.
24. Run the custom plant without changing Rust source code.

> **Strongest architectural acceptance criterion:** A new refinery/unit configuration must be constructible entirely from the equipment/template/configuration system without adding hardcoded unit-specific branches to the simulation engine.

---

## 38. Final Engineering Principle

**Build the platform, not the refinery.**

The engine must understand:

> equipment, ports, streams, states, physics, controls, tags, alarms, utilities, topology, history, failures, and scenarios.

NHT, TDHT, AH Unibon, Platforming, CCR, CDU, RFCC and future refinery units must simply be **reusable configurations/packages** built from those primitives.

This guarantees that NebuDCS can evolve from a DCS training simulator into a generic process engineering sandbox capable of modeling custom refinery designs and observing their simulated dynamic consequences.
