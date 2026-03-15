pub type Result<T> = anyhow::Result<T>;

pub mod archive;
pub mod cli;
pub mod map;
pub mod error;
pub mod walker;
pub mod hasher;
