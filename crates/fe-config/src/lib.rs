//! RorisDB Configuration and System Variables
//!
//! This crate provides configuration file loading (TOML format) and
//! system variable management with global and session scope support.

pub mod config;
pub mod variables;

pub use config::RorisConfig;
pub use variables::{
    GlobalVariables, SessionVariables, SystemVariableManager, VarDef, VarKind, VarScope,
};
