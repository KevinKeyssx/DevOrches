use std::collections::HashMap;
use serde::{ Serialize, Deserialize };

#[derive( Debug, Clone, Serialize, Deserialize )]
pub struct Instance {
	pub id      : String,
	pub name    : String,
	pub command : String,
	pub cwd     : Option<String>,
	pub env     : HashMap<String, String>,
	pub path    : String,
}

#[derive( Debug, Clone, Serialize, Deserialize )]
pub struct CustomInstance {
	pub name        : String,
	pub path        : String,
	pub script_name : String,
	pub command     : String,
}

#[derive( Debug, Clone, Serialize, Deserialize )]
pub struct WorkflowStep {
	pub name               : String,
	pub path               : String,
	pub script_name        : String,
	pub fail_on_error      : bool,
	#[serde( default = "default_background_delay" )]
	pub background_delay   : u64,
	pub env                : Option<HashMap<String, String>>,
}

fn default_background_delay() -> u64 {
	0
}

#[derive( Debug, Clone, Serialize, Deserialize )]
pub struct Workflow {
	pub name      : String,
	pub instances : Option<Vec<CustomInstance>>,
	pub steps     : Vec<WorkflowStep>,
}

