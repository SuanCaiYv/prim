use std::sync::Arc;

use ahash::AHashMap;

use crate::entity::{Msg, EXTENSION_THRESHOLD, PAYLOAD_THRESHOLD};

pub mod client;
pub mod server;

pub const BODY_SIZE: usize = EXTENSION_THRESHOLD + PAYLOAD_THRESHOLD;
pub const ALPN_PRIM: &[&[u8]] = &[b"prim"];
pub type InnerStates = AHashMap<String, InnerStatesValue>;

pub struct GenericParameterMap(pub AHashMap<&'static str, Box<dyn GenericParameter>>);

pub trait GenericParameter: Send + Sync + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_mut_any(&mut self) -> &mut dyn std::any::Any;
}

impl GenericParameterMap {
    pub fn get_parameter<T: GenericParameter + 'static>(&self) -> Option<&T> {
        match self.0.get(std::any::type_name::<T>()) {
            Some(parameter) => match parameter.as_any().downcast_ref::<T>() {
                Some(parameter) => Some(parameter),
                None => None,
            },
            None => None,
        }
    }

    pub fn get_parameter_mut<T: GenericParameter + 'static>(&mut self) -> Option<&mut T> {
        match self.0.get_mut(std::any::type_name::<T>()) {
            Some(parameter) => match parameter.as_mut_any().downcast_mut::<T>() {
                Some(parameter) => Some(parameter),
                None => None,
            },
            None => None,
        }
    }

    pub fn put_parameter<T: GenericParameter + 'static>(&mut self, parameter: T) {
        self.0
            .insert(std::any::type_name::<T>(), Box::new(parameter));
    }
}

pub enum InnerStatesValue {
    Str(String),
    Num(u64),
    Bool(bool),
    NumList(Vec<u64>),
    LastAck(Arc<Msg>),
    NumListMap(AHashMap<u64, Vec<u64>>),
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

    pub fn is_num_list(&self) -> bool {
        matches!(*self, InnerStatesValue::NumList(_))
    }

    pub fn as_num_list(&self) -> Option<&Vec<u64>> {
        match *self {
            InnerStatesValue::NumList(ref value) => Some(value),
            _ => None,
        }
    }

    pub fn as_mut_num_list(&mut self) -> Option<&mut Vec<u64>> {
        match *self {
            InnerStatesValue::NumList(ref mut value) => Some(value),
            _ => None,
        }
    }

    pub fn is_last_ack(&self) -> bool {
        matches!(*self, InnerStatesValue::LastAck(_))
    }

    pub fn as_last_ack(&self) -> Option<&Arc<Msg>> {
        match *self {
            InnerStatesValue::LastAck(ref value) => Some(value),
            _ => None,
        }
    }

    pub fn as_mut_last_ack(&mut self) -> Option<&mut Arc<Msg>> {
        match *self {
            InnerStatesValue::LastAck(ref mut value) => Some(value),
            _ => None,
        }
    }

    pub fn is_num_list_map(&self) -> bool {
        matches!(*self, InnerStatesValue::NumListMap(_))
    }

    pub fn as_num_list_map(&self) -> Option<&AHashMap<u64, Vec<u64>>> {
        match *self {
            InnerStatesValue::NumListMap(ref value) => Some(value),
            _ => None,
        }
    }

    pub fn as_mut_num_list_map(&mut self) -> Option<&mut AHashMap<u64, Vec<u64>>> {
        match *self {
            InnerStatesValue::NumListMap(ref mut value) => Some(value),
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
