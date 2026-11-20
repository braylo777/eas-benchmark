//! Rust port (pyCrust) of `eas/`: `engine.py` -> [`engine`], `stats.py` -> [`stats`],
//! `scenarios.py` -> [`scenarios`]. The torch probe code (`model_runner.py`, `run_probe.py`)
//! stays Python: it drives Hugging Face models.

pub mod engine;
pub mod scenarios;
pub mod stats;
