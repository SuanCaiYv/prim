use std::any::type_name;

use ahash::AHashMap;
use anyhow::anyhow;

pub mod client;
pub mod server;

use crate::{
    entity::{EXTENSION_THRESHOLD, PAYLOAD_THRESHOLD},
    Result,
};

pub const BODY_SIZE: usize = EXTENSION_THRESHOLD + PAYLOAD_THRESHOLD;
pub const ALPN_PRIM: &[&[u8]] = &[b"prim"];
pub type InnerStates = AHashMap<String, InnerStatesValue>;

pub struct GenericParameterMap(pub AHashMap<&'static str, Box<dyn GenericParameter>>);

pub trait GenericParameter: Send + Sync + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_mut_any(&mut self) -> &mut dyn std::any::Any;
}

impl GenericParameterMap {
    pub fn get_parameter<T: GenericParameter + 'static>(&self) -> Result<&T> {
        match self.0.get(std::any::type_name::<T>()) {
            Some(parameter) => match parameter.as_any().downcast_ref::<T>() {
                Some(parameter) => Ok(parameter),
                None => Err(anyhow!("parameter type mismatch")),
            },
            None => Err(anyhow!("parameter: {} not found", type_name::<T>())),
        }
    }

    pub fn get_parameter_mut<T: GenericParameter + 'static>(&mut self) -> Result<&mut T> {
        match self.0.get_mut(std::any::type_name::<T>()) {
            Some(parameter) => match parameter.as_mut_any().downcast_mut::<T>() {
                Some(parameter) => Ok(parameter),
                None => Err(anyhow!("parameter type mismatch")),
            },
            None => Err(anyhow!("parameter not found")),
        }
    }

    pub fn put_parameter<T: GenericParameter + 'static>(&mut self, parameter: T) {
        self.0
            .insert(std::any::type_name::<T>(), Box::new(parameter));
    }
}

pub enum InnerStatesValue {
    #[allow(unused)]
    Str(String),
    #[allow(unused)]
    Num(u64),
    #[allow(unused)]
    Bool(bool),
    #[allow(unused)]
    GenericParameterMap(GenericParameterMap),
}

impl InnerStatesValue {
    pub fn is_bool(&self) -> bool {
        matches!(*self, InnerStatesValue::Bool(_))
    }

    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            InnerStatesValue::Bool(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_mut_bool(&mut self) -> Option<&mut bool> {
        match *self {
            InnerStatesValue::Bool(ref mut value) => Some(value),
            _ => None,
        }
    }

    pub fn is_num(&self) -> bool {
        matches!(*self, InnerStatesValue::Num(_))
    }

    pub fn as_num(&self) -> Option<u64> {
        match *self {
            InnerStatesValue::Num(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_mut_num(&mut self) -> Option<&mut u64> {
        match *self {
            InnerStatesValue::Num(ref mut value) => Some(value),
            _ => None,
        }
    }

    pub fn is_str(&self) -> bool {
        matches!(*self, InnerStatesValue::Str(_))
    }

    pub fn as_str(&self) -> Option<&str> {
        match *self {
            InnerStatesValue::Str(ref value) => Some(value),
            _ => None,
        }
    }

    pub fn as_mut_str(&mut self) -> Option<&mut String> {
        match *self {
            InnerStatesValue::Str(ref mut value) => Some(value),
            _ => None,
        }
    }

    pub fn is_generic_parameter_map(&self) -> bool {
        matches!(*self, InnerStatesValue::GenericParameterMap(_))
    }

    pub fn as_generic_parameter_map(&self) -> Option<&GenericParameterMap> {
        match *self {
            InnerStatesValue::GenericParameterMap(ref value) => Some(value),
            _ => None,
        }
    }

    pub fn as_mut_generic_parameter_map(&mut self) -> Option<&mut GenericParameterMap> {
        match *self {
            InnerStatesValue::GenericParameterMap(ref mut value) => Some(value),
            _ => None,
        }
    }
}
