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
pub struct WaitCondition {
	pub port      : Option<u16>,
	pub log_match : Option<String>,
	pub delay_ms  : Option<u64>,
}

#[derive( Debug, Clone, Serialize, Deserialize )]
pub struct WorkflowStep {
	pub instance_id : String,
	pub wait_for    : Option<WaitCondition>,
}

#[derive( Debug, Clone, Serialize, Deserialize )]
pub struct WorkflowPhase {
	pub name     : String,
	pub parallel : bool,
	pub steps    : Vec<WorkflowStep>,
}

#[derive( Debug, Clone, Serialize, Deserialize )]
pub struct Workflow {
	pub version   : String,
	pub name      : Option<String>,
	pub env       : HashMap<String, String>,
	pub instances : Vec<Instance>,
	pub phases    : Vec<WorkflowPhase>,
}
