#![no_std]

pub mod contract;
pub mod error;
pub mod royalty;
pub mod approval;
pub mod storage;

#[cfg(test)]
mod test;

pub use contract::ClipsContract;
pub use error::ClipsError;
