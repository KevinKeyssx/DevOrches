mod models;
mod storage;
mod commands;

use tauri::Manager;

// Intervalo de actualización en segundos para el monitoreo de recursos
const STATS_MONITOR_INTERVAL_SECS: u64 = 2;

#[cfg_attr( mobile, tauri::mobile_entry_point )]
pub fn run() {
	tauri::Builder::default()
		.setup( | app | {
			if cfg!( debug_assertions ) {
				app.handle().plugin(
					tauri_plugin_log::Builder::default()
						.level( log::LevelFilter::Info )
						.build(),
				)?;
			}

			// Inicializamos el plugin de diálogos nativos
			app.handle().plugin( tauri_plugin_dialog::init() )?;

			let db_state = storage::db::DbState::new( app.handle() )?;
			app.manage( db_state );

			// Inicializamos y registramos el estado de los procesos activos
			let process_state = commands::project::ProcessState::default();
			let active_processes = process_state.active_processes.clone();
			app.manage( process_state );

			// Spawn a background task to monitor resources of active processes
			let app_handle = app.handle().clone();
			tauri::async_runtime::spawn( async move {
				use sysinfo::{ System, Pid, ProcessRefreshKind };
				use tauri::Emitter;

				let mut sys = System::new_all();

				loop {
					tokio::time::sleep( tokio::time::Duration::from_secs( STATS_MONITOR_INTERVAL_SECS ) ).await;

					let targets: Vec<( String, u32 )> = {
						if let Ok( guard ) = active_processes.lock() {
							guard.iter().map( | ( id, proc ) | ( id.clone(), proc.pid ) ).collect()
						} else {
							Vec::new()
						}
					};

					if targets.is_empty() {
						continue;
					}

					sys.refresh_processes_specifics( ProcessRefreshKind::new().with_cpu().with_memory() );
					sys.refresh_memory();

					let total_system_memory = sys.total_memory();

					// Construimos el mapa de dependencias padre-hijo una sola vez para este tick
					let mut parent_to_children = std::collections::HashMap::new();
					for ( pid, process ) in sys.processes() {
						if let Some( ppid ) = process.parent() {
							parent_to_children.entry( ppid ).or_insert_with( Vec::new ).push( *pid );
						}
					}

					for ( instance_id, parent_pid ) in targets {
						let root_pid = Pid::from( parent_pid as usize );
						let mut tree_pids = std::collections::HashSet::new();
						tree_pids.insert( root_pid );

						let mut queue = vec![ root_pid ];
						let mut i = 0;
						while i < queue.len() {
							let current = queue[ i ];
							i += 1;
							if let Some( children ) = parent_to_children.get( &current ) {
								for &child in children {
									if tree_pids.insert( child ) {
										queue.push( child );
									}
								}
							}
						}

						let mut tree_cpu = 0.0;
						let mut tree_memory = 0;
						for pid in &tree_pids {
							if let Some( process ) = sys.process( *pid ) {
								tree_cpu += process.cpu_usage();
								tree_memory += process.memory();
							}
						}

						let _ = app_handle.emit( "instance-stats", commands::project::StatsPayload {
							instance_id,
							cpu				: tree_cpu,
							memory			: tree_memory,
							total_memory	: total_system_memory,
						} );
					}
				}
			} );

			Ok( () )
		} )
		.invoke_handler( tauri::generate_handler![
			commands::project::detect_project_instances,
			commands::project::add_project,
			commands::project::get_projects,
			commands::project::delete_project,
			commands::project::select_directory,
			commands::project::save_log_file,
			commands::project::add_project_path,
			commands::project::delete_project_instance,
			commands::project::start_instance,
			commands::project::stop_instance,
			commands::project::update_instance,
			commands::project::add_manual_instance,
			commands::project::load_workflow,
			commands::project::save_workflow,
			commands::project::export_workflow,
			commands::project::import_workflow,
			commands::project::run_workflow,
			commands::project::abort_workflow,
		] )
		.run( tauri::generate_context!() )
		.expect( "error while running tauri application" );
}
