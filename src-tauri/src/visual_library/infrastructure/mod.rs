//! Compatibility infrastructure boundary. Implementations are migrated here
//! incrementally while callers depend only on `application`.

pub mod legacy_adapter;
pub mod providers;
pub mod qa;
pub mod sqlite;
pub mod storage;
