//! Rust trial harness for the EAW benchmark (v0.2).
//!
//! * [`engine`], [`scenarios`], [`stats`]: ports of `eas/engine.py`, `eas/scenarios.py`, `eas/stats.py`.
//! * [`pyrandom`], [`probe_data`]: bit-exact port of Python's `random.Random` and the statement set.
//! * [`montecarlo`]: the simulation runner (Study S).
//! * [`ollama`], [`behaviour`]: the local-model behaviour runner (Study B).
//! * [`boot`]: bootstrap CIs, AUC, McNemar, Holm, seed-variance summaries.
//!
//! The torch hidden-state probe (`scripts/run_probe.py`) stays Python: it drives Hugging Face models.

pub mod behaviour;
pub mod boot;
pub mod cli;
pub mod engine;
pub mod items;
pub mod metrics;
pub mod montecarlo;
pub mod ollama;
pub mod probe_data;
pub mod pyrandom;
pub mod scenarios;
pub mod stats;
