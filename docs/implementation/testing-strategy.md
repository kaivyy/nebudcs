---
title: Testing Strategy
version: 1.0.0
status: draft
date: 2026-09-05
---

# PHILOSOPHY
Test-first for every phase. Tests precede implementation.

# TEST CLASSIFICATION
- `crates/<name>/src/*.rs` — inline unit tests (`#[cfg(test)]`)
- `crates/<name>/tests/` — integration tests
- `tests/` — workspace-level integration tests
- `benchmarks/` — criterion benchmarks

# PHASE 1 TESTS
**`nebudcs-types`**
- `test_entity_ids_are_unique`
- `test_equipment_id_not_assignable_to_stream_id`
- `test_simulation_time_monotonic`
- `test_schema_version_ordering`
- `test_error_display`

**`nebudcs-units`**
- `test_pressure_pascal_identity`
- `test_pressure_bar_conversion`
- `test_pressure_psi_conversion`
- `test_temperature_celsius_conversion`
- `test_temperature_fahrenheit_conversion`
- `test_round_trip_conversion`
- `test_speed_rpm_to_rad_per_sec`
- `test_serde_round_trip_pressure`
- `test_unit_aware_value_quantity_kind`

**`nebudcs-model`**
- `test_equipment_definition_serde_json`
- `test_equipment_definition_serde_toml`
- `test_equipment_instance_references_definition`
- `test_project_hierarchy`
- `test_stream_definition_connects_ports`
- `test_tag_value_variants`
- `test_tag_quality_good_and_bad`
- `test_port_domain_variants_serde`
- `test_validation_result_severity_ordering`
- `test_pump_definition_fixture`
- `test_vessel_definition_fixture`
- `test_stream_composition_not_boolean`

# INTEGRATION TESTS
- `test_project_file_round_trip`
- `test_pump_definition_load_from_file`

# DETERMINISM TESTS
- `test_same_inputs_same_outputs`
- `test_replay_matches_original`

# ARCHITECTURE TESTS
- `test_types_crate_has_no_workspace_dependencies`
- `test_units_crate_depends_only_on_types`
- `test_model_crate_depends_only_on_types_and_units`
- `test_no_dependency_cycles`

# PERFORMANCE TESTS
- `bench_simulation_tick_100_equipment`
- `bench_simulation_tick_1000_equipment`
- `bench_historian_write_throughput`
- `bench_project_load_medium`

# CI PIPELINE DEFINITION
- On PR: `fmt`, `clippy`, `test`, `deny check`.
- Nightly: `bench`, coverage.

# FIXTURE FILES
- `tests/fixtures/pump_definition.toml`
- `tests/fixtures/vessel_definition.toml`
- `tests/fixtures/minimal_project.toml`
