//! Simulation system integration bridge connecting physics models to the double-buffered runtime engine.

use std::collections::BTreeMap;

use nebudcs_sim::{SimError, SimulationSystem, StateReadView, StateWriteView, StepContext};
use nebudcs_types::{EquipmentId, TagId};
use nebudcs_units::Density;

use crate::equipment::{AccumulatorVolumeModel, GenericEquipmentPhysics};

/// Generic physics simulation system implementing [`nebudcs_sim::SimulationSystem`].
///
/// Dispatches dynamic fixed-step updates for registered equipment models (accumulators,
/// resistances, head sources, and custom capabilities) during engine tick cycles
/// and updates double-buffered equipment state and DCS tags.
#[derive(Debug, Default)]
pub struct GenericPhysicsSystem {
    equipment: BTreeMap<EquipmentId, GenericEquipmentPhysics>,
}

impl GenericPhysicsSystem {
    /// Creates an empty generic physics system.
    pub fn new() -> Self {
        Self {
            equipment: BTreeMap::new(),
        }
    }

    /// Registers a generic equipment container with arbitrary capabilities.
    pub fn register_equipment(&mut self, equipment: GenericEquipmentPhysics) {
        self.equipment.insert(equipment.id(), equipment);
    }

    /// Convenience registration for a tank or accumulator volume with level tag publishing.
    pub fn register_tank(
        &mut self,
        equipment_id: EquipmentId,
        level_tag: TagId,
        total_volume: f64,
        cross_sectional_area: f64,
        initial_mass: f64,
        fluid_density: f64,
    ) {
        let density = Density::new_unchecked(fluid_density);
        let model =
            AccumulatorVolumeModel::new(total_volume, cross_sectional_area, initial_mass, density);

        let mut eq = GenericEquipmentPhysics::new(equipment_id);
        eq.add_capability(model);
        eq.bind_tag("level", level_tag);

        self.equipment.insert(equipment_id, eq);
    }

    /// Returns a reference to an equipment instance by ID.
    pub fn equipment(&self, id: EquipmentId) -> Option<&GenericEquipmentPhysics> {
        self.equipment.get(&id)
    }

    /// Returns a mutable reference to an equipment instance by ID.
    pub fn equipment_mut(&mut self, id: EquipmentId) -> Option<&mut GenericEquipmentPhysics> {
        self.equipment.get_mut(&id)
    }

    /// Returns the number of registered equipment instances.
    pub fn len(&self) -> usize {
        self.equipment.len()
    }

    /// Returns true if no equipment is registered.
    pub fn is_empty(&self) -> bool {
        self.equipment.is_empty()
    }
}

impl SimulationSystem for GenericPhysicsSystem {
    fn name(&self) -> &str {
        "GenericPhysicsSystem"
    }

    fn step(
        &mut self,
        ctx: StepContext,
        _read_state: &StateReadView<'_>,
        write_state: &mut StateWriteView<'_>,
    ) -> Result<(), SimError> {
        let dt = ctx.dt;

        for (eq_id, eq) in &mut self.equipment {
            if let Err(e) = eq.evaluate(dt) {
                return Err(SimError::SystemExecutionFailed {
                    system_name: "GenericPhysicsSystem".to_string(),
                    detail: format!("physics step failed for equipment {eq_id}: {e}"),
                });
            }

            // Update State[N+1] equipment variables for all known exported variables
            let _ = write_state.mutate_equipment(*eq_id, |eq_state| {
                for var_name in &[
                    "mass",
                    "level",
                    "bottom_pressure",
                    "headspace_pressure",
                    "flow",
                    "delta_p",
                    "duty",
                    "speed",
                    "opening",
                ] {
                    if let Some(val) = eq.get_variable(var_name) {
                        eq_state.set_var(*var_name, val);
                    }
                }
            });

            // Update State[N+1] DCS tags bound to variables
            for (var_name, tag_id) in eq.tag_bindings() {
                if let Some(val) = eq.get_variable(var_name) {
                    write_state.set_tag_float(*tag_id, val);
                }
            }
        }

        Ok(())
    }
}
