use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use std::collections::HashMap;
use serde_json::Value;
use tauri::{ State, Emitter };
use uuid::Uuid;
use chrono::Local;
use tauri_plugin_dialog::DialogExt;
use std::sync::{ Arc, Mutex };
use tokio::io::AsyncReadExt;
use std::process::Stdio;

use crate::models::project::Project;
use crate::models::workflow::Instance;
use crate::storage::db::DbState;

pub struct ActiveProcess {
	pub pid : u32,
}

#[derive( Default )]
pub struct ProcessState {
	pub active_processes : Arc<Mutex<HashMap<String, ActiveProcess>>>,
}

#[derive( Clone, serde::Serialize )]
pub struct LogPayload {
	pub instance_id : String,
	pub text        : String,
}

#[derive( Clone, serde::Serialize )]
pub struct StatusPayload {
	pub instance_id : String,
	pub running     : bool,
	pub exit_code   : Option<i32>,
}

#[tauri::command]
pub async fn select_directory( app: tauri::AppHandle ) -> Result<Option<String>, String> {
	let folder = app.dialog()
		.file()
		.blocking_pick_folder()
		.and_then( | path | path.into_path().ok() )
		.and_then( | path_buf | path_buf.to_str().map( | s | s.to_string() ) );

	Ok( folder )
}

#[tauri::command]
pub async fn save_log_file(
	app          : tauri::AppHandle,
	default_name : String,
	content      : String,
) -> Result<bool, String> {
	let file_path = app.dialog()
		.file()
		.add_filter( "Log files", &[ "log", "txt" ] )
		.set_file_name( &default_name )
		.blocking_save_file();

	match file_path {
		Some( path ) => {
			let path_buf = path.into_path()
				.map_err( | e | format!( "Error al obtener la ruta: {}", e ) )?;

			std::fs::write( &path_buf, content.as_bytes() )
				.map_err( | e | format!( "Error al escribir el archivo: {}", e ) )?;

			Ok( true )
		}
		None => Ok( false ),
	}
}

pub async fn detect_path_instances( path: &str ) -> Result<Vec<Instance>, String> {
	let project_path = Path::new( path );
	if !project_path.exists() {
		return Err( format!( "El directorio no existe: {}", path ) );
	}

	let package_json_path = project_path.join( "package.json" );
	if !package_json_path.exists() {
		return Ok( Vec::new() );
	}

	let file = File::open( &package_json_path )
		.map_err( | err | format!( "No se pudo abrir package.json: {}", err ) )?;
	let reader = BufReader::new( file );

	let json: Value = serde_json::from_reader( reader )
		.map_err( | err | format!( "Error al parsear package.json: {}", err ) )?;

	let mut instances = Vec::new();

	if let Some( scripts ) = json.get( "scripts" ).and_then( | s | s.as_object() ) {
		for ( name, cmd_value ) in scripts {
			if let Some( _cmd ) = cmd_value.as_str() {
				let pm = if project_path.join( "pnpm-lock.yaml" ).exists() {
					"pnpm"
				} else if project_path.join( "yarn.lock" ).exists() {
					"yarn"
				} else {
					"npm"
				};

				let full_command = format!( "{} run {}", pm, name );
				let unique_id = format!( "{}#{}", path, name );

				instances.push( Instance {
					id      : unique_id,
					name    : name.clone(),
					command : full_command,
					cwd     : None,
					env     : HashMap::new(),
					path    : path.to_string(),
				} );
			}
		}
	}

	Ok( instances )
}

#[tauri::command]
pub async fn detect_project_instances( path: String ) -> Result<Vec<Instance>, String> {
	detect_path_instances( &path ).await
}

#[tauri::command]
pub async fn start_instance(
	app         : tauri::AppHandle,
	state       : State<'_, ProcessState>,
	db_state    : State<'_, DbState>,
	project_id  : String,
	instance_id : String,
) -> Result<(), String> {
	let ( command_str, path_str ) = {
		let data_guard = db_state.data.read()
			.map_err( | _ | "Error de concurrencia al leer la base de datos" )?;

		let project = data_guard.projects.iter().find( | p | p.id == project_id )
			.ok_or_else( || "No se encontró el proyecto especificado".to_string() )?;

		let instance = project.instances.iter().find( | i | i.id == instance_id )
			.ok_or_else( || "No se encontró la instancia especificada".to_string() )?;

		( instance.command.clone(), instance.path.clone() )
	};

	{
		let guard = state.active_processes.lock()
			.map_err( | _ | "Error al bloquear el estado de procesos" )?;
		if guard.contains_key( &instance_id ) {
			return Err( "La instancia ya está en ejecución".to_string() );
		}
	}

	let mut tokio_cmd = if cfg!( windows ) {
		let mut cmd = tokio::process::Command::new( "cmd" );
		cmd.args( &[ "/C", &command_str ] );
		cmd
	} else {
		let mut cmd = tokio::process::Command::new( "sh" );
		cmd.args( &[ "-c", &command_str ] );
		cmd
	};

	tokio_cmd.current_dir( &path_str );
	tokio_cmd.stdout( Stdio::piped() );
	tokio_cmd.stderr( Stdio::piped() );

	let mut child = tokio_cmd.spawn()
		.map_err( | err | format!( "No se pudo iniciar el comando: {}", err ) )?;

	let pid = child.id().ok_or_else( || "No se pudo obtener el PID del proceso".to_string() )?;

	{
		let mut guard = state.active_processes.lock()
			.map_err( | _ | "Error al bloquear el estado de procesos" )?;
		guard.insert( instance_id.clone(), ActiveProcess { pid } );
	}

	let _ = app.emit( "instance-status", StatusPayload {
		instance_id : instance_id.clone(),
		running     : true,
		exit_code   : None,
	} );

	let stdout = child.stdout.take().ok_or( "No se pudo capturar stdout" )?;
	let stderr = child.stderr.take().ok_or( "No se pudo capturar stderr" )?;

	let app_stdout = app.clone();
	let inst_id_stdout = instance_id.clone();
	tokio::spawn( async move {
		let mut buffer = [ 0u8; 1024 ];
		let mut reader = stdout;
		loop {
			match reader.read( &mut buffer ).await {
				Ok( 0 ) => break,
				Ok( n ) => {
					let text = String::from_utf8_lossy( &buffer[ ..n ] ).to_string();
					let _ = app_stdout.emit( "instance-log", LogPayload {
						instance_id : inst_id_stdout.clone(),
						text,
					} );
				}
				Err( _ ) => break,
			}
		}
	} );

	let app_stderr = app.clone();
	let inst_id_stderr = instance_id.clone();
	tokio::spawn( async move {
		let mut buffer = [ 0u8; 1024 ];
		let mut reader = stderr;
		loop {
			match reader.read( &mut buffer ).await {
				Ok( 0 ) => break,
				Ok( n ) => {
					let text = String::from_utf8_lossy( &buffer[ ..n ] ).to_string();
					let _ = app_stderr.emit( "instance-log", LogPayload {
						instance_id : inst_id_stderr.clone(),
						text,
					} );
				}
				Err( _ ) => break,
			}
		}
	} );

	let active_processes_clone = state.active_processes.clone();
	let app_monitor = app.clone();
	let inst_id_monitor = instance_id.clone();
	tokio::spawn( async move {
		let exit_status = child.wait().await;

		{
			if let Ok( mut guard ) = active_processes_clone.lock() {
				guard.remove( &inst_id_monitor );
			}
		}

		let code = exit_status.ok().and_then( | s | s.code() );
		let _ = app_monitor.emit( "instance-status", StatusPayload {
			instance_id : inst_id_monitor,
			running     : false,
			exit_code   : code,
		} );
	} );

	Ok( () )
}

#[tauri::command]
pub async fn stop_instance(
	state       : State<'_, ProcessState>,
	instance_id : String,
) -> Result<(), String> {
	let mut guard = state.active_processes.lock()
		.map_err( | _ | "Error de concurrencia al acceder al estado de procesos" )?;

	if let Some( proc ) = guard.remove( &instance_id ) {
		if cfg!( windows ) {
			let mut kill_cmd = std::process::Command::new( "taskkill" );
			kill_cmd.args( &[ "/F", "/T", "/PID", &proc.pid.to_string() ] );
			let _ = kill_cmd.spawn();
		} else {
			let mut kill_cmd = std::process::Command::new( "kill" );
			kill_cmd.args( &[ "-9", &proc.pid.to_string() ] );
			let _ = kill_cmd.spawn();
		}
		Ok( () )
	} else {
		Err( "El proceso no está en ejecución".to_string() )
	}
}

#[tauri::command]
pub async fn update_instance(
	state       : State<'_, DbState>,
	project_id  : String,
	instance_id : String,
	new_name    : String,
	new_command : String,
) -> Result<(), String> {
	{
		let mut data_guard = state.data.write()
			.map_err( | _ | "Error de concurrencia al escribir en la base de datos" )?;

		if let Some( project ) = data_guard.projects.iter_mut().find( | p | p.id == project_id ) {
			if let Some( inst ) = project.instances.iter_mut().find( | i | i.id == instance_id ) {
				inst.name = new_name;
				inst.command = new_command;
			} else {
				return Err( "No se encontró la instancia especificada".to_string() );
			}
			project.updated_at = Local::now().to_rfc3339();
		} else {
			return Err( "No se encontró el proyecto especificado".to_string() );
		}
	}

	state.save()?;
	Ok( () )
}

#[tauri::command]
pub async fn add_manual_instance(
	state      : State<'_, DbState>,
	project_id : String,
	path       : String,
	name       : String,
	command    : String,
) -> Result<Instance, String> {
	if name.trim().is_empty() || command.trim().is_empty() {
		return Err( "El nombre y el comando no pueden estar vacíos".to_string() );
	}

	let unique_id = format!( "{}#{}", path, name );
	let new_instance = Instance {
		id      : unique_id,
		name,
		command,
		cwd     : None,
		env     : HashMap::new(),
		path    : path.clone(),
	};

	{
		let mut data_guard = state.data.write()
			.map_err( | _ | "Error de concurrencia al escribir en la base de datos" )?;

		if let Some( project ) = data_guard.projects.iter_mut().find( | p | p.id == project_id ) {
			if project.instances.iter().any( | i | i.id == new_instance.id ) {
				return Err( "Ya existe una instancia con ese nombre en esta ruta".to_string() );
			}
			project.instances.push( new_instance.clone() );
			project.updated_at = Local::now().to_rfc3339();
		} else {
			return Err( "No se encontró el proyecto especificado".to_string() );
		}
	}

	state.save()?;
	Ok( new_instance )
}

#[tauri::command]
pub async fn add_project(
	state : State<'_, DbState>,
	name  : String,
	paths : Vec<String>,
) -> Result<Project, String> {
	let mut instances = Vec::new();

	for path in &paths {
		let detected = detect_path_instances( path ).await?;
		instances.extend( detected );
	}

	let new_project = Project {
		id         : Uuid::new_v4().to_string(),
		name,
		paths,
		instances,
		created_at : Local::now().to_rfc3339(),
		updated_at : Local::now().to_rfc3339(),
	};

	{
		let mut data_guard = state.data.write()
			.map_err( | _ | "Error de concurrencia al escribir en la base de datos" )?;

		if data_guard.projects.iter().any( | p | p.name == new_project.name ) {
			return Err( "Ya existe un proyecto con ese nombre".to_string() );
		}

		data_guard.projects.push( new_project.clone() );
	}

	state.save()?;

	Ok( new_project )
}

#[tauri::command]
pub async fn add_project_path(
	state      : State<'_, DbState>,
	project_id : String,
	path       : String,
) -> Result<Vec<Instance>, String> {
	let project_path = Path::new( &path );
	if !project_path.exists() {
		return Err( format!( "El directorio no existe: {}", path ) );
	}

	let detected = detect_path_instances( &path ).await?;
	if detected.is_empty() {
		return Err( "No se encontraron scripts ejecutables en esta ruta (falta package.json)".to_string() );
	}

	let mut new_instances = Vec::new();
	let mut path_already_exists = false;

	{
		let mut data_guard = state.data.write()
			.map_err( | _ | "Error de concurrencia al escribir en la base de datos" )?;

		if let Some( project ) = data_guard.projects.iter_mut().find( | p | p.id == project_id ) {
			if project.paths.contains( &path ) {
				path_already_exists = true;
			} else {
				project.paths.push( path.clone() );
			}

			for inst in detected {
				if !project.instances.iter().any( | existing | existing.id == inst.id ) {
					new_instances.push( inst.clone() );
					project.instances.push( inst );
				}
			}

			project.updated_at = Local::now().to_rfc3339();
		} else {
			return Err( "No se encontró el proyecto especificado".to_string() );
		}
	}

	if new_instances.is_empty() {
		if path_already_exists {
			return Err( "ALREADY_REGISTERED".to_string() );
		} else {
			return Err( "No se encontraron nuevas instancias para agregar".to_string() );
		}
	}

	state.save()?;

	Ok( new_instances )
}

#[tauri::command]
pub async fn delete_project_instance(
	state       : State<'_, DbState>,
	project_id  : String,
	instance_id : String,
) -> Result<(), String> {
	{
		let mut data_guard = state.data.write()
			.map_err( | _ | "Error de concurrencia al escribir en la base de datos" )?;

		if let Some( project ) = data_guard.projects.iter_mut().find( | p | p.id == project_id ) {
			let initial_len = project.instances.len();
			project.instances.retain( | inst | inst.id != instance_id );

			if project.instances.len() == initial_len {
				return Err( "No se encontró la instancia especificada en el proyecto".to_string() );
			}

			project.updated_at = Local::now().to_rfc3339();
		} else {
			return Err( "No se encontró el proyecto especificado".to_string() );
		}
	}

	state.save()?;

	Ok( () )
}

#[tauri::command]
pub async fn get_projects( state: State<'_, DbState> ) -> Result<Vec<Project>, String> {
	let data_guard = state.data.read()
		.map_err( | _ | "Error de concurrencia al leer la base de datos" )?;

	Ok( data_guard.projects.clone() )
}

#[tauri::command]
pub async fn delete_project( state: State<'_, DbState>, id: String ) -> Result<(), String> {
	{
		let mut data_guard = state.data.write()
			.map_err( | _ | "Error de concurrencia al escribir en la base de datos" )?;

		let initial_len = data_guard.projects.len();
		data_guard.projects.retain( | p | p.id != id );

		if data_guard.projects.len() == initial_len {
			return Err( "No se encontró el proyecto con el ID especificado".to_string() );
		}
	}

	state.save()?;

	Ok( () )
}
