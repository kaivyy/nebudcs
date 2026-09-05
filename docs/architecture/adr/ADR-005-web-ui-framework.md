---
title: ADR-005: Web UI Framework
status: accepted
date: 2026-09-05
author: NebuDCS Architecture Team
---

# ADR-005: Web UI Framework

## Status

Accepted

## Context

The Web Operator/Training Station requires a high-performance web interface to display process graphics, faceplates, alarms, and trends. The PRD explicitly advises avoiding React unless proven necessary, favoring lightweight, framework-free architectures or Web Components.

## Problem

Which framework should be used to build the web-based HMI and operator station?

## Decision

We will use **Web Components + Lit** with **TypeScript**, utilizing **SVG** for process graphics and **Canvas/WebGL** for complex or high-frequency rendering components (like trends).

*Technology Version:* Lit 3.0+, TypeScript 5.5+ (as of 2026).
*Compile Cost:* Fast (esbuild/Vite).
*Bundle Size Impact:* Extremely small (~5-10KB for Lit core).
*Runtime/Memory Overhead:* Minimal; leverages native browser APIs.

## Alternatives Considered

- **React**: Heavy runtime overhead, virtual DOM diffing is inefficient for high-frequency tag updates (e.g., 1000s of values changing at 10Hz). Explicitly discouraged by PRD.
- **Svelte / SolidJS**: Excellent performance and reactivity. SolidJS provides fine-grained reactivity perfect for HMI values, but requires a build step and ecosystem buy-in.
- **Vanilla Web Components**: No dependencies, but tedious to write boilerplate for attributes and shadow DOM.

## Rationale

Lit provides a thin, lightweight wrapper over native Web Components, offering reactivity without a Virtual DOM. This is ideal for an HMI where thousands of SVG elements or DOM nodes (tags, faceplate values) need to update based on WebSocket data. Web Components ensure UI elements (like a Pump Faceplate) are reusable and encapsulated, aligning perfectly with the equipment template architecture.

## Trade-offs

- Smaller ecosystem of pre-built complex components compared to React.
- State management for the entire application must be handled carefully (likely via standard RxJS or signals) to avoid prop-drilling.

## Consequences

- The web UI will be incredibly fast and lightweight.
- UI components can be easily embedded in other contexts or customer portals.
- The architecture forces a clean separation of data (WebSocket streams) and presentation.

## Rejected Alternatives

- React was rejected due to PRD constraints and Virtual DOM overhead for high-frequency updates.
- Vanilla Web Components were rejected due to developer ergonomics; Lit provides the necessary ergonomics with minimal cost.

## Future Reconsideration Criteria

If the application state becomes overwhelmingly complex and Lit's reactivity model proves insufficient to manage it cleanly, we may reconsider SolidJS due to its superior fine-grained signal architecture while maintaining high performance.
