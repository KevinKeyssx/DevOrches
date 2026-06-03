use std::fs::{ self, File };
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::RwLock;
use tauri::{ AppHandle, Manager };
use crate::models::project::Project;

#[derive( Debug, serde::Serialize, serde::Deserialize, Clone )]
pub struct AppDatabase {
	pub projects : Vec<Project>,
}

impl Default for AppDatabase {
	fn default() -> Self {
		AppDatabase {
			projects : Vec::new(),
		}
	}
}

pub struct DbState {
	pub db_path : PathBuf,
	pub data    : RwLock<AppDatabase>,
}

impl DbState {
	pub fn new( app_handle: &AppHandle ) -> Result<Self, String> {
		let app_dir = app_handle.path().app_data_dir()
			.map_err( | err | format!( "No se pudo obtener el directorio de la app: {}", err ) )?;

		if !app_dir.exists() {
			fs::create_dir_all( &app_dir )
				.map_err( | err | format!( "No se pudo crear el directorio de datos: {}", err ) )?;
		}

		let db_path = app_dir.join( "db.json" );
		let database = if db_path.exists() {
			let file = File::open( &db_path )
				.map_err( | err | format!( "No se pudo abrir db.json: {}", err ) )?;
			let reader = BufReader::new( file );
			serde_json::from_reader( reader )
				.unwrap_or_else( | _ | AppDatabase::default() )
		} else {
			let db = AppDatabase::default();
			let content = serde_json::to_string_pretty( &db )
				.map_err( | err | format!( "Error al serializar DB por defecto: {}", err ) )?;
			fs::write( &db_path, content )
				.map_err( | err | format!( "Error al escribir db.json inicial: {}", err ) )?;
			db
		};

		Ok( DbState {
			db_path,
			data : RwLock::new( database ),
		} )
	}

	pub fn save( &self ) -> Result<(), String> {
		let data_guard = self.data.read()
			.map_err( | _ | "Error de concurrencia al leer la base de datos para guardado" )?;

		let content = serde_json::to_string_pretty( &*data_guard )
			.map_err( | err | format!( "Error al serializar la base de datos: {}", err ) )?;

		let tmp_path = self.db_path.with_extension( "tmp" );
		fs::write( &tmp_path, content )
			.map_err( | err | format!( "Error al escribir archivo temporal: {}", err ) )?;

		fs::rename( &tmp_path, &self.db_path )
			.map_err( | err | format!( "Error al renombrar archivo temporal: {}", err ) )?;

		Ok( () )
	}
}
