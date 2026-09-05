---
title: Training Architecture
version: 0.1.0
status: draft
date: 2026-09-05
---

# Training Architecture

## 1. Overview

The Training layer in NebuDCS orchestrates operator training, evaluation, and instructor control. It sits entirely decoupled from the UI layer—training logic never evaluates specific HMI button coordinates or screen interactions. Instead, it evaluates the logical operations performed against the simulation state (via tags).

## 2. Training Layer Design

The training module manages:
- **Objectives:** The overarching goals of the session (e.g., "Safely start up the distillation column").
- **Expected Operator Actions:** Key operations the trainee must perform (e.g., "Set controller TIC-201 to AUTO").
- **Process-Response Criteria:** Verifying the plant reaches a desired state (e.g., "Bottoms temperature stable at 250°C for 5 minutes").
- **Alarm-Response Criteria:** Evaluating how quickly and appropriately an operator acknowledges and mitigates alarms.
- **Forbidden Actions:** Rules that instantly fail or penalize the trainee (e.g., "Opening bypass valve XV-101 while main pump is running").
- **Time Limits:** Constraining how long the operator has to achieve an objective or respond to a fault.

Training operates against the real simulation state. There are no "scripted movies" or predetermined outcomes; if the operator makes a mistake, the physics engine will simulate the resulting plant upset.

## 3. Instructor Station

The Instructor Station is a specialized client (Web or Desktop) that allows the instructor to:
- Select and launch scenarios.
- Monitor trainee actions in real-time.
- Inject ad-hoc faults or adjust process disturbances outside of the scripted scenario.
- Freeze, pause, resume, or accelerate simulation time.
- Take snapshots of the current state to use as new initial conditions.
- Observe a consolidated view of objective completion and operator scoring.

## 4. Assessment and Grading

Grading in NebuDCS is completely objective and criteria-based:
- **Action-Based:** Did the operator change the correct tag to the correct state within the time window?
- **Process-Based:** Did the plant remain within safe operating limits (e.g., no safety relief valves popped, no HIHI alarms triggered)?
- **Efficiency-Based:** How much product was off-spec during a transition?

## 5. Architecture Diagrams

### Training Session Lifecycle

```mermaid
graph TD
    A[Instructor Selects Scenario] --> B[Initialize Simulation from Snapshot]
    B --> C[Trainee Logs In]
    C --> D[Session Active]
    D --> E{Instructor / Engine Intervention}
    E -->|Time/Event| F[Inject Scripted Faults]
    E -->|Manual| G[Inject Ad-Hoc Faults]
    F --> H[Trainee Responds via HMI]
    G --> H
    H --> I[Evaluate Objectives & Criteria]
    I --> J{End Condition Met?}
    J -- No --> D
    J -- Yes --> K[End Session]
    K --> L[Generate Training Report / Replay]
```

### Objective Evaluation Flow

```mermaid
graph LR
    A[Operator HMI] -->|Tag Write Command| B[Protocol Layer]
    B --> C[Simulation Core]
    C --> D[Tag State Updated]
    D --> E[Training Engine]
    E -->|Check against| F[Scenario Objectives]
    F -->|Match found| G[Mark Objective Complete]
    F -->|Forbidden action| H[Deduct Points / Trigger Failure]
```

### Instructor / Trainee Architecture

```mermaid
graph TD
    subgraph Instructor Client
        A[Instructor Dashboard]
        B[Fault Control Panel]
        C[Live Assessment View]
    end
    
    subgraph Trainee Client
        D[Operator HMI]
        E[Alarm Summary]
        F[Faceplates]
    end
    
    subgraph Server / Core
        G[Simulation Core]
        H[Scenario Engine]
        I[Training/Assessment Engine]
        J[Historian]
    end
    
    A -->|Start/Pause/Load| H
    B -->|Inject Fault| G
    D <-->|Read/Write Tags| G
    G --> I
    I --> C
    H --> I
    G --> J
```
