use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use std::collections::HashMap;
use serde_json::Value;
use tauri::{ State, Emitter, Manager };
use uuid::Uuid;
use chrono::Local;
use tauri_plugin_dialog::DialogExt;
use std::sync::{ Arc, Mutex };
use tokio::io::AsyncReadExt;
use std::process::Stdio;

use crate::models::project::Project;
use crate::models::workflow::{ Instance, Workflow, WorkflowStep };
use crate::storage::db::DbState;

pub struct ActiveProcess {
	pub pid : u32,
}

#[derive( Default )]
pub struct ProcessState {
	pub active_processes   : Arc<Mutex<HashMap<String, ActiveProcess>>>,
	pub workflow_cancelled : Arc<std::sync::atomic::AtomicBool>,
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

#[derive( Clone, serde::Serialize )]
pub struct StatsPayload {
	pub instance_id  : String,
	pub cpu          : f32,
	pub memory       : u64,
	pub total_memory : u64,
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
	let ( project_name, root_path ) = {
		let data_guard = db_state.data.read()
			.map_err( | _ | "Error de concurrencia al leer la base de datos" )?;

		let project = data_guard.projects.iter().find( | p | p.id == project_id )
			.ok_or_else( || "No se encontró el proyecto especificado".to_string() )?;

		( project.name.clone(), project.paths.first().cloned().unwrap_or_default() )
	};

	let workflow = load_workflow_internal( &app, &project_name );

	let ( command_str, path_str, envs_to_inject ) = {
		let data_guard = db_state.data.read()
			.map_err( | _ | "Error de concurrencia al leer la base de datos" )?;

		let project = data_guard.projects.iter().find( | p | p.id == project_id )
			.ok_or_else( || "No se encontró el proyecto especificado".to_string() )?;

		let mut resolved = None;

		// 1. Try to find in database
		if let Some( inst ) = project.instances.iter().find( | i | i.id == instance_id ) {
			resolved = Some( ( inst.command.clone(), inst.path.clone() ) );
		}

		// 2. Try to find in workflow custom instances
		if resolved.is_none() {
			if let Some( ref wf ) = workflow {
				if let Some( ref custom_instances ) = wf.instances {
					let parts: Vec<&str> = instance_id.split( '#' ).collect();
					if parts.len() == 2 {
						let target_abs_path = parts[ 0 ];
						let script_name = parts[ 1 ];

						if let Some( ci ) = custom_instances.iter().find( | ci | {
							ci.script_name == script_name && paths_are_equivalent( &target_abs_path, &std::path::Path::new( &root_path ).join( &ci.path ).to_string_lossy() )
						} ) {
							resolved = Some( ( ci.command.clone(), target_abs_path.to_string() ) );
						}
					}
				}
			}
		}

		let ( cmd, path ) = resolved.ok_or_else( || "No se encontró la instancia especificada".to_string() )?;

		// 3. Find if there are env variables in workflow steps for this instance
		let mut envs = HashMap::new();
		if let Some( ref wf ) = workflow {
			let parts: Vec<&str> = instance_id.split( '#' ).collect();
			if parts.len() == 2 {
				let target_abs_path = parts[ 0 ];
				let script_name = parts[ 1 ];

				if let Some( step ) = wf.steps.iter().find( | s | {
					s.script_name == script_name && paths_are_equivalent( &target_abs_path, &std::path::Path::new( &root_path ).join( &s.path ).to_string_lossy() )
				} ) {
					if let Some( ref step_envs ) = step.env {
						envs = step_envs.clone();
					}
				}
			}
		}

		( cmd, path, envs )
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

	// Inject envs
	for ( k, v ) in envs_to_inject {
		tokio_cmd.env( k, v );
	}

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

fn normalize_to_unix_path( path_str: &str ) -> String {
	path_str.replace( '\\', "/" )
}

fn normalize_workflow_paths( mut workflow: Workflow ) -> Workflow {
	for step in &mut workflow.steps {
		step.path = normalize_to_unix_path( &step.path );
	}
	if let Some( ref mut instances ) = workflow.instances {
		for inst in instances {
			inst.path = normalize_to_unix_path( &inst.path );
		}
	}
	workflow
}

fn paths_are_equivalent( path1: &str, path2: &str ) -> bool {
	let p1 = std::path::Path::new( path1 );
	let p2 = std::path::Path::new( path2 );
	match ( p1.canonicalize(), p2.canonicalize() ) {
		( Ok( c1 ), Ok( c2 ) ) => c1 == c2,
		_                      => p1 == p2,
	}
}

fn load_workflow_internal( app: &tauri::AppHandle, project_name: &str ) -> Option<Workflow> {
	let app_data = app.path().app_data_dir().ok()?;
	let path = app_data.join( "projects" ).join( project_name ).join( "workflow.yml" );
	if !path.exists() {
		return None;
	}
	let file_content = std::fs::read_to_string( &path ).ok()?;
	let mut workflow: Workflow = serde_yaml::from_str( &file_content ).ok()?;
	workflow = normalize_workflow_paths( workflow );
	Some( workflow )
}

#[tauri::command]
pub async fn load_workflow(
	app        : tauri::AppHandle,
	db_state   : State<'_, DbState>,
	project_id : String,
) -> Result<Option<Workflow>, String> {
	let project_name = {
		let guard = db_state.data.read()
			.map_err( | _ | "Error de concurrencia al leer la base de datos" )?;
		let project = guard.projects.iter().find( | p | p.id == project_id )
			.ok_or_else( || "No se encontró el proyecto".to_string() )?;
		project.name.clone()
	};

	Ok( load_workflow_internal( &app, &project_name ) )
}

#[tauri::command]
pub async fn save_workflow(
	app        : tauri::AppHandle,
	db_state   : State<'_, DbState>,
	project_id : String,
	workflow   : Workflow,
) -> Result<(), String> {
	let project_name = {
		let guard = db_state.data.read()
			.map_err( | _ | "Error de concurrencia al leer la base de datos" )?;
		let project = guard.projects.iter().find( | p | p.id == project_id )
			.ok_or_else( || "No se encontró el proyecto".to_string() )?;
		project.name.clone()
	};

	let app_data = app.path().app_data_dir()
		.map_err( | e | format!( "No se pudo obtener el directorio de la app: {}", e ) )?;
	let path = app_data.join( "projects" ).join( &project_name ).join( "workflow.yml" );

	if let Some( parent ) = path.parent() {
		std::fs::create_dir_all( parent )
			.map_err( | e | format!( "No se pudo crear el directorio de destino: {}", e ) )?;
	}

	let normalized = normalize_workflow_paths( workflow );
	let yaml_content = serde_yaml::to_string( &normalized )
		.map_err( | e | format!( "Error al serializar el flujo a YAML: {}", e ) )?;

	std::fs::write( &path, yaml_content )
		.map_err( | e | format!( "Error al guardar el archivo de flujo: {}", e ) )?;

	Ok( () )
}

#[tauri::command]
pub async fn export_workflow(
	app        : tauri::AppHandle,
	db_state   : State<'_, DbState>,
	project_id : String,
	workflow   : Workflow,
) -> Result<bool, String> {
	let project_name = {
		let guard = db_state.data.read()
			.map_err( | _ | "Error de concurrencia al leer la base de datos" )?;
		let project = guard.projects.iter().find( | p | p.id == project_id )
			.ok_or_else( || "No se encontró el proyecto".to_string() )?;
		project.name.clone()
	};

	let filename = format!( "{}.workflow.yml", project_name );

	let file_path = app.dialog()
		.file()
		.add_filter( "YAML files", &[ "yml", "yaml" ] )
		.set_file_name( &filename )
		.blocking_save_file();

	match file_path {
		Some( path ) => {
			let path_buf = path.into_path()
				.map_err( | e | format!( "Error al obtener la ruta de guardado: {}", e ) )?;

			let normalized = normalize_workflow_paths( workflow );
			let yaml_content = serde_yaml::to_string( &normalized )
				.map_err( | e | format!( "Error al serializar el flujo: {}", e ) )?;

			std::fs::write( &path_buf, yaml_content.as_bytes() )
				.map_err( | e | format!( "Error al escribir el archivo: {}", e ) )?;

			Ok( true )
		}
		None => Ok( false ),
	}
}

#[tauri::command]
pub async fn import_workflow(
	app        : tauri::AppHandle,
	db_state   : State<'_, DbState>,
	project_id : String,
) -> Result<Option<Workflow>, String> {
	let file_path = app.dialog()
		.file()
		.add_filter( "YAML files", &[ "yml", "yaml" ] )
		.blocking_pick_file();

	match file_path {
		Some( path ) => {
			let path_buf = path.into_path()
				.map_err( | e | format!( "Error al obtener la ruta: {}", e ) )?;

			let file_content = std::fs::read_to_string( &path_buf )
				.map_err( | e | format!( "Error al leer el archivo: {}", e ) )?;

			let mut workflow: Workflow = serde_yaml::from_str( &file_content )
				.map_err( | e | format!( "Error al parsear el YAML: {}", e ) )?;

			workflow = normalize_workflow_paths( workflow );

			let project_name = {
				let guard = db_state.data.read()
					.map_err( | _ | "Error de concurrencia al leer la base de datos" )?;
				let project = guard.projects.iter().find( | p | p.id == project_id )
					.ok_or_else( || "No se encontró el proyecto".to_string() )?;
				project.name.clone()
			};

			let app_data = app.path().app_data_dir()
				.map_err( | e | format!( "No se pudo obtener el directorio de la app: {}", e ) )?;
			let dest_path = app_data.join( "projects" ).join( &project_name ).join( "workflow.yml" );

			if let Some( parent ) = dest_path.parent() {
				std::fs::create_dir_all( parent )
					.map_err( | e | format!( "No se pudo crear el directorio de destino: {}", e ) )?;
			}

			let yaml_content = serde_yaml::to_string( &workflow )
				.map_err( | e | format!( "Error al serializar el flujo: {}", e ) )?;

			std::fs::write( &dest_path, yaml_content )
				.map_err( | e | format!( "Error al guardar el archivo: {}", e ) )?;

			Ok( Some( workflow ) )
		}
		None => Ok( None ),
	}
}

#[derive( Clone, serde::Serialize )]
pub struct WorkflowStatusPayload {
	pub step_index : usize,
	pub status     : String,
	pub exit_code  : Option<i32>,
}

pub async fn execute_step_process(
	app         : tauri::AppHandle,
	project_id  : &str,
	step        : &WorkflowStep,
	env_context : &HashMap<String, String>,
	workflow    : &Workflow,
) -> Result<Option<i32>, String> {
	let state = app.state::<ProcessState>();
	let db_state = app.state::<DbState>();

	let ( command_str, path_str, instance_id ) = {
		let data_guard = db_state.data.read()
			.map_err( | _ | "Error de concurrencia al leer la base de datos" )?;

		let project = data_guard.projects.iter().find( | p | p.id == project_id )
			.ok_or_else( || "No se encontró el proyecto especificado".to_string() )?;

		if project.paths.is_empty() {
			return Err( "El proyecto no tiene rutas vinculadas".to_string() );
		}

		let root_path = &project.paths[ 0 ];
		let target_abs_path = std::path::Path::new( root_path ).join( &step.path );
		let target_abs_str = target_abs_path.to_string_lossy().to_string();

		let mut resolved = None;

		if let Some( ref custom_instances ) = workflow.instances {
			if let Some( ci ) = custom_instances.iter().find( | ci | {
				ci.script_name == step.script_name && paths_are_equivalent( &target_abs_str, &std::path::Path::new( root_path ).join( &ci.path ).to_string_lossy() )
			} ) {
				resolved = Some( ( ci.command.clone(), target_abs_str.clone() ) );
			}
		}

		if resolved.is_none() {
			if let Some( inst ) = project.instances.iter().find( | i | {
				i.name == step.script_name && paths_are_equivalent( &i.path, &target_abs_str )
			} ) {
				resolved = Some( ( inst.command.clone(), inst.path.clone() ) );
			}
		}

		let ( cmd, path ) = resolved.ok_or_else( || format!( "No se encontró el comando para el script '{}' en '{}'", step.script_name, step.path ) )?;
		let inst_id = format!( "{}#{}", path, step.script_name );

		( cmd, path, inst_id )
	};

	{
		let guard = state.active_processes.lock()
			.map_err( | _ | "Error al bloquear el estado de procesos" )?;
		if guard.contains_key( &instance_id ) {
			return Err( format!( "La instancia '{}' ya está en ejecución", instance_id ) );
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

	for ( k, v ) in env_context {
		tokio_cmd.env( k, v );
	}

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

	let exit_status = child.wait().await;

	{
		if let Ok( mut guard ) = state.active_processes.lock() {
			guard.remove( &instance_id );
		}
	}

	let code = exit_status.ok().and_then( | s | s.code() );
	let _ = app.emit( "instance-status", StatusPayload {
		instance_id : instance_id.clone(),
		running     : false,
		exit_code   : code,
	} );

	Ok( code )
}

#[tauri::command]
pub async fn run_workflow(
	app        : tauri::AppHandle,
	state      : State<'_, ProcessState>,
	db_state   : State<'_, DbState>,
	project_id : String,
	workflow   : Workflow,
) -> Result<(), String> {
	state.workflow_cancelled.store( false, std::sync::atomic::Ordering::SeqCst );

	tauri::async_runtime::spawn( async move {
		let process_state = app.state::<ProcessState>();
		let mut env_context = HashMap::new();

		for ( index, step ) in workflow.steps.iter().enumerate() {
			if process_state.workflow_cancelled.load( std::sync::atomic::Ordering::SeqCst ) {
				let _ = app.emit( "workflow-status", "cancelled".to_string() );
				return;
			}

			let _ = app.emit( "workflow-step-status", WorkflowStatusPayload {
				step_index : index,
				status     : "running".to_string(),
				exit_code  : None,
			} );

			if let Some( ref step_envs ) = step.env {
				for ( k, v ) in step_envs {
					env_context.insert( k.clone(), v.clone() );
				}
			}

			let run_result = execute_step_process(
				app.clone(),
				&project_id,
				step,
				&env_context,
				&workflow,
			).await;

			match run_result {
				Ok( exit_code ) => {
					let success = exit_code == Some( 0 );

					let status_str = if success { "success".to_string() } else { "failed".to_string() };
					let _ = app.emit( "workflow-step-status", WorkflowStatusPayload {
						step_index : index,
						status     : status_str,
						exit_code,
					} );

					if !success && step.fail_on_error {
						let _ = app.emit( "workflow-status", "failed".to_string() );
						return;
					}
				}
				Err( _ ) => {
					let _ = app.emit( "workflow-step-status", WorkflowStatusPayload {
						step_index : index,
						status     : "failed".to_string(),
						exit_code  : None,
					} );

					if step.fail_on_error {
						let _ = app.emit( "workflow-status", "failed".to_string() );
						return;
					}
				}
			}
		}

		if process_state.workflow_cancelled.load( std::sync::atomic::Ordering::SeqCst ) {
			let _ = app.emit( "workflow-status", "cancelled".to_string() );
		} else {
			let _ = app.emit( "workflow-status", "success".to_string() );
		}
	} );

	Ok( () )
}

#[tauri::command]
pub async fn abort_workflow(
	state    : State<'_, ProcessState>,
	workflow : Workflow,
) -> Result<(), String> {
	state.workflow_cancelled.store( true, std::sync::atomic::Ordering::SeqCst );

	let mut guard = state.active_processes.lock()
		.map_err( | _ | "Error de concurrencia al acceder al estado de procesos" )?;

	for step in &workflow.steps {
		let suffix = format!( "#{}", step.script_name );
		let pids_to_kill: Vec<( String, u32 )> = guard.iter()
			.filter( | ( id, _ ) | id.ends_with( &suffix ) )
			.map( | ( id, proc ) | ( id.clone(), proc.pid ) )
			.collect();

		for ( id, pid ) in pids_to_kill {
			guard.remove( &id );
			if cfg!( windows ) {
				let mut kill_cmd = std::process::Command::new( "taskkill" );
				kill_cmd.args( &[ "/F", "/T", "/PID", &pid.to_string() ] );
				let _ = kill_cmd.spawn();
			} else {
				let mut kill_cmd = std::process::Command::new( "kill" );
				kill_cmd.args( &[ "-9", &pid.to_string() ] );
				let _ = kill_cmd.spawn();
			}
		}
	}
	Ok( () )
}
