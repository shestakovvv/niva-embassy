#![no_std]

pub mod analog_input;
pub mod digital_input;
mod server;
pub use server::modbus_can_server;
pub use server::modbus_server;
pub use server::modbus_master;
pub mod flash_memory;