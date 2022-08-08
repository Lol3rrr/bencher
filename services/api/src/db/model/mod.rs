pub mod adapter;
pub mod benchmark;
pub mod branch;
pub mod perf;
pub mod project;
pub mod report;
pub mod testbed;
pub mod user;
pub mod version;

// https://docs.rs/chrono/latest/chrono/naive/struct.NaiveDateTime.html#impl-Display-for-NaiveDateTime
pub const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.f";
