//! Frozen pre-migration instruction selection; test-only equivalence oracle.
#![allow(dead_code)]
use super::{Target, abi, data, heap, ir, machine, output, string, utf8};
mod arguments;
mod array;
mod conversion;
pub(super) mod emit;
mod failure;
mod input;
mod rounding;
mod unsigned;
