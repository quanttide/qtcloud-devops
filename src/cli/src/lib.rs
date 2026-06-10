pub mod code;
pub mod git;
pub mod release;

// 向后兼容：保持 commands 和 model 路径可用，委托到新模块
pub use git::submodule;

#[cfg(feature = "python")]
pub mod python;
