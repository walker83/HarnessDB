//! PostgreSQL wire protocol v3 compatibility layer for HarnessDB.
//!
//! This crate implements the PostgreSQL wire protocol v3, enabling standard
//! PostgreSQL clients (psql, psycopg2, JDBC) to connect to HarnessDB.
//! This is the foundation for Hologres protocol compatibility.
//!
//! # Protocol Overview
//! - Transport: TCP binary (PostgreSQL wire protocol v3)
//! - Authentication: MD5 or SCRAM-SHA-256
//! - Query modes: Simple Query and Extended Query (Parse/Bind/Execute)
//! - Hologres uses this protocol with AccessKey-based authentication

pub mod auth;
pub mod catalog;
pub mod connection;
pub mod message;
pub mod server;

pub use server::{PgServer, PgServerConfig};
