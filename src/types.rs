use crate::{util, Config};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;

pub type BoolResult = Result<bool, Box<dyn Error>>;
pub type ConfigResult = Result<Config, Box<dyn Error>>;
pub type HashMapResult = Result<HashMap<String, Option<String>>, Box<dyn Error>>;
pub type HashSetResult = Result<HashSet<String>, Box<dyn Error>>;
pub type OptionVecString = Option<Vec<String>>;
pub type PathBufResult = Result<PathBuf, Box<dyn Error>>;
pub type PromptOptionResult = Result<util::PromptOption, Box<dyn Error>>;
pub type StringResult = Result<String, Box<dyn Error>>;
pub type UnitResult = Result<(), Box<dyn Error>>;
pub type VecPathBufResult = Result<Vec<PathBuf>, Box<dyn Error>>;
