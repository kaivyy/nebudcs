---
title: ADR-008: Network Protocol (WebSocket + REST)
status: accepted
date: 2026-09-05
author: NebuDCS Architecture Team
---

# ADR-008: Network Protocol (WebSocket + REST)

## Status

Accepted

## Context

The Web Operator Station and remote clients need to communicate with the NebuDCS server. They must retrieve project structure, send configuration changes, and receive high-frequency telemetry data (tag updates, alarms) for the HMI.

## Problem

How should the network communication between the simulation engine and external clients be structured?

## Decision

We will use **REST/HTTP** for project configuration and state initialization, and **WebSockets** for real-time telemetry and control actions.
- The WebSocket payload for telemetry will use a batched **binary protocol** (e.g., MessagePack or bincode) to minimize bandwidth and parsing overhead.
- Telemetry will strictly use **delta updates**; clients subscribe to tags, and the server only sends values when they change beyond a deadband.

## Alternatives Considered

- **gRPC**: Excellent for typed RPCs and streaming, but browser support requires gRPC-Web (which has limitations and proxy requirements). Heavy setup.
- **Server-Sent Events (SSE) + REST**: SSE is great for one-way telemetry, but doesn't handle two-way low-latency control actions (like an operator clicking a pump start button) as elegantly as a single WebSocket connection.
- **Pure JSON over WebSocket**: Easy to debug, but parsing JSON strings at 20Hz for 10,000 tags will crush the browser's JavaScript garbage collector and consume massive bandwidth.

## Rationale

REST is the standard for CRUD operations (creating projects, loading definitions). WebSockets provide full-duplex, low-latency communication ideal for DCS operation. Binary serialization via MessagePack/bincode avoids string allocation and parsing overhead in both Rust and the browser, essential for meeting the performance targets. The subscription model ensures clients only receive data for the screens they are currently viewing.

## Trade-offs

- Binary protocols are harder to debug via network inspectors than JSON.
- Implementing robust subscription and delta-tracking logic on the server adds complexity.

## Consequences

- The HMI will remain highly responsive even on slow networks.
- The server must maintain state regarding what each WebSocket client is subscribed to and what the last sent value was (to calculate deltas).

## Rejected Alternatives

- Pure JSON was rejected due to performance and GC overhead.
- gRPC was rejected due to browser interoperability friction.

## Future Reconsideration Criteria

If WebTransport becomes universally supported and stabilized, we may migrate from WebSockets to WebTransport over HTTP/3 for better handling of head-of-line blocking in high-latency network conditions.
