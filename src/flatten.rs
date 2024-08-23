use alloc::vec::Vec;

pub trait ExtFieldFlattener<From, To> : Sized {
    fn flatten(from: &From) -> Vec<To>;
}