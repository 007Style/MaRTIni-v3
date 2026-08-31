# MaRTIni v3 Protocol SDK

> WINLab, Rutgers University — Daneyand Singley, Roland Wunderlich, Ramnath Ravindran

## Overview

This document describes the three authoring paths for handoff protocol plugins in MaRTIni v3.

## 1. Native Rust Trait

Implement `HandoffProtocol` from `src/sim/protocol.rs`, compile it into the binary.

## 2. WASM Plugin

Compile a `.wasm` module exposing the `decide_handoff` ABI, drop it in `plugins/`.
Requires the `wasm-plugins` Cargo feature.

## 3. Declarative TOML

Write a TOML file matching the schema in `plugins/example-threshold.toml`.
No code required — the built-in rule engine evaluates it at runtime.

---

*Full ABI specification to be filled in during Sub-Task 9.*
