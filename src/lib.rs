//! The library for the jodin compiler/interpreter

#![deny(missing_docs)]
#![deny(unused_imports)]
#![deny(unused_mut)]

#[macro_use]
extern crate clap;

#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate lazy_static;

pub mod cli;
pub mod compilation_settings;
pub mod core;
pub mod parsing;
pub mod passes;
pub mod utility;
