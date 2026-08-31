//! Configuration module — grid, speed, radio, terrain, and session configs.

pub mod grid;
pub mod radio;
pub mod session;
pub mod speed;
pub mod terrain;

pub use grid::GridConfig;
pub use radio::{RadioConfig, RadioTechnology, TrafficProfile};
pub use session::SimSession;
pub use speed::SpeedConfig;
pub use terrain::{TerrainConfig, TerrainType};
