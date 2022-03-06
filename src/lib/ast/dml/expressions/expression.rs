use std::fmt::Debug;

use dyn_clone::{clone_trait_object, DynClone};

pub trait IExpresstion: DynClone + Debug {}

clone_trait_object!(IExpresstion);

#[derive(Clone, Debug)]
pub struct Expresstion {}
