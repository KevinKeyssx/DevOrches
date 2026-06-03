use serde::{ Serialize, Deserialize };
use crate::models::workflow::Instance;

#[derive( Debug, Clone, Serialize, Deserialize )]
pub struct Project {
	pub id         : String,
	pub name       : String,
	pub paths      : Vec<String>,
	pub instances  : Vec<Instance>,
	pub created_at : String,
	pub updated_at : String,
}
