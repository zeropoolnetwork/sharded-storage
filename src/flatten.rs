use alloc::vec::Vec;


use p3_field::extension::{
    Complex, ComplexExtendable
};
use p3_field::{ExtensionField, Field};



pub trait ExtFieldFlattener<From, To> : Sized {
    fn flatten(from: &From) -> Vec<To>;
}


pub struct ExtensionFlattener;

impl <From:ExtensionField<To>, To:Field> ExtFieldFlattener<From, To> for ExtensionFlattener 
{
    fn flatten(from: &From) -> Vec<To> {
        from.as_base_slice().to_vec()
    }
}

pub struct ComplexExtensionFlattener;

impl <From:ExtensionField<Complex<To>>, To:ComplexExtendable> ExtFieldFlattener<From, To> for ComplexExtensionFlattener 
{
    fn flatten(from: &From) -> Vec<To> {
        from.as_base_slice().iter().flat_map(|x| x.to_array().into_iter()).collect()
    }
}
