use crate::Config;
use std::error::Error;

pub type BoolResult = Result<Option<bool>, Box<dyn Error>>;
pub type ConfigResult = Result<Config, Box<dyn Error>>;
pub type StringBoolResult = Result<(String, bool), Box<dyn Error>>;
pub type StringOptionResult = Result<Option<String>, Box<dyn Error>>;
pub type StringResult = Result<String, Box<dyn Error>>;
pub type UnitResult = Result<(), Box<dyn Error>>;
