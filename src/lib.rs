pub mod api;
pub mod config;
pub mod database;
pub mod logging;
pub mod model;
pub mod repl;
pub mod service;

mod macros;

pub trait Located {
    fn location(&self) -> snafu::Location;
}
