// Data sharding library according to paper 
// [Efficient Data Distribution with Reed-Solomon Codes for Sharded Storage](https://ethresear.ch/t/20232)

#![no_std]

extern crate alloc;


mod symmetric_cfft;


pub use symmetric_cfft::*;