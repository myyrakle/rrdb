use std::fmt::Debug;

use dyn_clone::{clone_trait_object, DynClone};

pub trait IExpression: DynClone + Debug {}

clone_trait_object!(IExpression);

#[derive(Clone, Debug)]
pub struct Expression {}
