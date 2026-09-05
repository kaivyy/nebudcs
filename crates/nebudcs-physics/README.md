# nebudcs-physics

Generic process physics, property packages, conservation laws, and hydraulic solvers for NebuDCS.

## 1. Architectural Principles

- **Generic & Extensible**: No hardcoded process units (`CDU`, `NHT`, `CCR`) or equipment classes (`Pump.rs`, `Compressor.rs`, `Column.rs`).
- **Capability-Based Composition**: Equipment behavior is constructed by composing standard or custom [`EquipmentCapability`] implementations into [`GenericEquipmentPhysics`].
- **Zero-Allocation Fixed Timestep Execution**: In double-buffered simulation ticks, systems read `State[N]` via [`StateReadView`] and write strictly to `State[N+1]` via [`StateWriteView`].
- **Dimensional & Physical Rigor**: Strict domain checking prevents negative absolute pressures, temperatures below absolute zero, non-normalized compositions, and unphysical phase/vapor fraction combinations.

## 2. Modules & Governing Physics

### 2.1 State & Composition (`state`)
- **[`PhysicalState`]**: Pressure ($P > 0$ Pa), thermodynamic temperature ($T > 0$ K), density ($\rho > 0$ kg/m³), specific enthalpy ($h$ J/kg), vapor fraction ($\beta \in [0.0, 1.0]$), phase (`Liquid`, `Vapor`, `TwoPhase`), and component composition.
- **[`ComponentComposition`]**: Normalized multi-component mass or mole fractions ($\sum w_i = 1.0$ or $\sum y_i = 1.0$) with explicit [`CompositionBasis`] conversion routines:
  $$w_i = \frac{y_i M_{w,i}}{\sum_j y_j M_{w,j}}, \quad y_i = \frac{w_i / M_{w,i}}{\sum_j (w_j / M_{w,j})}$$

### 2.2 Property Packages (`properties`)
- **[`PropertyPackage`]**: Trait evaluating physical states from $(P, T, \mathbf{z})$.
- **[`IncompressibleLiquid`]**: Constant or temperature-dependent liquid density and specific heat capacity.
- **[`IdealGasMixture`]**:
  - Molecular weight: $M_w = \sum_i y_i M_{w,i}$ (mole basis) or $M_w = \left[\sum_i \frac{w_i}{M_{w,i}}\right]^{-1}$ (mass basis).
  - Density from ideal gas law: $\rho = \frac{P M_w}{R T}$.
  - Specific heat capacity & enthalpy: converts mole fractions to mass fractions $w_i$ before integrating $h(T) = \sum_i w_i C_{p,mass,i} (T - T_{ref})$ for dimensional correctness.
- **[`SaturatedFluidPackage`]**: Rigorous VLE property package:
  - Saturation pressure: $P_{sat}(T) = P_{ref} \exp\left[ \frac{\Delta H_{vap} M_w}{R} \left( \frac{1}{T_b} - \frac{1}{T} \right) \right]$.
  - Saturation temperature: $T_{sat}(P) = \left[ \frac{1}{T_b} - \frac{R \ln(P / P_{ref})}{\Delta H_{vap} M_w} \right]^{-1}$.
  - Isobaric $P-h$ flash calculation [`flash_from_enthalpy`] evaluating subcooled liquid, two-phase mixture ($\beta = \frac{h - h_L}{\Delta H_{vap}}$), and superheated vapor.

### 2.3 Hydraulics & Flow Elements (`hydraulics`)
- **Churchill Friction Factor**: Unified friction factor valid continuously across laminar ($Re < 2300$), critical transition ($2300 \le Re \le 4000$), and fully turbulent ($Re > 4000$) regimes:
  $$f = 8 \left[ \left(\frac{8}{Re}\right)^{12} + \frac{1}{(A + B)^{1.5}} \right]^{1/12}$$
- **Darcy-Weisbach with Hydrostatic Elevation**:
  $$\Delta P = \text{sign}(\dot{m}) \left( f \frac{L}{D} \frac{\rho v^2}{2} \right) + \rho g \Delta z$$
  Uses $C^1$ smooth regularization near zero flow to guarantee numerical stability and returns signed [`DifferentialPressure`].
- **Control Valve Hydraulics**: Standard industrial $C_v$ relationship:
  $$\dot{m} = x_{open} \cdot C_v \cdot N_v \cdot \sqrt{\rho \cdot |\Delta P|}$$
  Includes bidirectional sign handling and monotonic inverse pressure drop calculation. Regularized below $\Delta P = 1.0$ Pa.

### 2.4 Conservation Laws (`conservation`)
- **Total Mass Balance**:
  $$\frac{dm}{dt} = \sum \dot{m}_{in} - \sum \dot{m}_{out}$$
- **Species Mass Balance**:
  $$\frac{d(m \cdot w_i)}{dt} = \sum (\dot{m}_{in} \cdot w_{i,in}) - \sum (\dot{m}_{out} \cdot w_i)$$
  Rejects basis mismatches and preserves composition basis.
- **Energy / Enthalpy Balance**:
  $$\frac{d(m \cdot h)}{dt} = \sum (\dot{m}_{in} \cdot h_{in}) - \sum (\dot{m}_{out} \cdot h) + \dot{Q} + \dot{W}_s$$

### 2.5 Solvers (`solver`)
- **[`RecycleSolver`]**: Non-linear fixed-point solver with:
  - Successive substitution with configurable under-relaxation damping $\lambda \in (0, 1]$
  - Bounded Wegstein acceleration with bounded extrapolation factor $q \in [-5.0, 0.0]$
  - Mixed absolute and relative convergence tolerances: $|y_i - x_i| \le \text{tol}_{abs} + \text{tol}_{rel} |x_i|$
  - Divergence guards ($> 10^{12}$), finite number checks, and strict maximum iteration failure reporting.

### 2.6 Equipment Capability Framework (`equipment` & `system`)
- **[`EquipmentCapability`]**: Modular physics behavior trait with evaluation context:
  - [`AccumulatorVolumeModel`]: Dynamic mass holdup, fractional level, bottom pressure ($P_{headspace} + \rho g L H$), species accumulation, and enthalpy accumulation.
  - [`HeadSourceModel`]: Quadratic head curve and affinity laws $\Delta P = \omega^2 a_0 + a_2 Q^2$.
  - [`FlowResistanceModel`]: Valve and restriction pressure drops.
  - [`HeatExchangeModel`]: Thermal duty $Q = UA \cdot \Delta T$ and counter-current LMTD.
- **[`GenericEquipmentPhysics`]**: Composite container binding capabilities to DCS tags.
- **[`GenericPhysicsSystem`]**: Double-buffered [`SimulationSystem`] stepping physics across ticks.

## 3. Extensibility Classification & Boundaries

- **Current Extensibility (Phase 4)**: Rust trait extension model. Third-party code and tests can construct arbitrary equipment models by implementing [`EquipmentCapability`] and adding them to [`GenericEquipmentPhysics`] without modifying any core simulation crates.
- **Future Custom Equipment Authoring (Phase 8+)**: Sandboxed scripting (Rhai / DSL) for runtime authoring without Rust compilation. Phase 4 establishes the clean capability abstractions that this future scripting engine will drive.
