---
title: ADR-004: Desktop UI Framework (egui and wgpu)
status: accepted
date: 2026-09-05
author: NebuDCS Architecture Team
---

# ADR-004: Desktop UI Framework (egui and wgpu)

## Status

Accepted

## Context

The NebuDCS Desktop Engineering Station is used for project creation, P&ID editing, and configuration. It requires a highly responsive, complex interface with custom drawing capabilities (for the P&ID canvas). The product vision explicitly bans Electron and embedded Chromium for the desktop engineering application.

## Problem

Which UI framework should be used for the native Desktop Engineering application?

## Decision

We will use **egui** (with `eframe`) for the immediate-mode UI, and **wgpu** for GPU-accelerated process graphics and the P&ID canvas.

*Technology Version:* egui 0.30+, wgpu 24.0+ (as of 2026).
*Compile Cost:* Moderate to high (wgpu pulls in graphics backends).
*Binary Size Impact:* Moderate (~15-30MB).
*Runtime/Memory Overhead:* Very low, highly efficient.

## Alternatives Considered

- **Electron / Tauri**: Electron is explicitly banned. Tauri is lighter, but still relies on web views, adding unnecessary IPC overhead and RAM usage for an engineering tool.
- **GTK / Qt**: Traditional retained-mode GUI toolkits. Heavy dependencies, complex build steps (especially on Windows), and difficult to integrate smoothly with custom high-performance Rust canvas drawing.
- **Slint**: Excellent declarative UI, but immediate-mode is often better suited for the highly dynamic, data-driven nature of property editors and graph canvases found in CAD/DCS software.

## Rationale

`egui` is written purely in Rust, compiles easily across platforms, and operates in immediate mode, which trivially binds to the dynamic simulation state without complex state synchronization logic. `wgpu` provides a safe, modern, cross-platform graphics API for rendering complex P&ID graphs at 60 FPS without stuttering.

## Trade-offs

- Immediate mode UI can consume more battery on laptops if not careful about repainting (we must implement reactive repaints).
- Styling in `egui` is less flexible than CSS.
- Accessibility (screen readers) in immediate-mode UIs is historically challenging, though improving.

## Consequences

- The desktop app will be a single, self-contained binary with no external DLL/runtime dependencies.
- Very low memory footprint compared to web-based alternatives.
- The UI code will be written entirely in Rust, sharing types directly with the simulation core.

## Rejected Alternatives

- Electron/Tauri rejected due to PRD constraints and memory overhead.
- GTK/Qt rejected due to build complexity and C++ interop friction.

## Future Reconsideration Criteria

If `egui` proves too inflexible for advanced styling requirements, or if accessibility becomes a strict regulatory requirement that `egui` cannot meet, we would re-evaluate Slint or a custom retained-mode UI.
