mod models;
mod storage;
mod commands;

use tauri::Manager;

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
			app.manage( commands::project::ProcessState::default() );

			Ok( () )
		} )
		.invoke_handler( tauri::generate_handler![
			commands::project::detect_project_instances,
			commands::project::add_project,
			commands::project::get_projects,
			commands::project::delete_project,
			commands::project::select_directory,
			commands::project::add_project_path,
			commands::project::delete_project_instance,
			commands::project::start_instance,
			commands::project::stop_instance,
			commands::project::update_instance,
			commands::project::add_manual_instance,
		] )
		.run( tauri::generate_context!() )
		.expect( "error while running tauri application" );
}
