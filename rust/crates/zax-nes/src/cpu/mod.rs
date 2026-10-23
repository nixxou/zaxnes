//! The 6502 core of the 2A03 and its bus contract (REQ-CORE-110, REQ-CORE-111).

mod addressing_mode;
mod cpu6502;
mod cpu_bus;
mod cpu_operation;
mod cpu_tracer;
mod opcode_table;

pub use addressing_mode::AddressingMode;
pub use cpu6502::Cpu6502;
pub use cpu_bus::CpuBus;
pub use cpu_operation::CpuOperation;
pub use cpu_tracer::{CpuTraceLine, CpuTracer};
pub use opcode_table::{OpcodeInfo, OpcodeTable, OperandAccess};
