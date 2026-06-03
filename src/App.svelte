<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';

	import type {
		Project,
		Instance
	}                   from './lib/types';
	import InstanceCard from './components/InstanceCard.svelte';
	import Console      from './components/Console.svelte';
	import Sidebar      from './components/Sidebar.svelte';
	import LogViewer    from './components/LogViewer.svelte';

	// Detección de ventana secundaria de log viewer
	const urlParams = new URLSearchParams( window.location.search );
	const logViewerKey = urlParams.get( 'logViewerKey' );

	interface LogViewerData {
		instanceName : string;
		logs         : string;
	}

	let logViewerData = $state<LogViewerData | null>( null );

	if ( logViewerKey ) {
		const raw = localStorage.getItem( logViewerKey );
		if ( raw ) {
			try {
				logViewerData = JSON.parse( raw ) as LogViewerData;
			} catch {
				logViewerData = null;
			}
		}
	}

	let projects        = $state<Project[]>( [] );
	let selectedProject = $state<Project | null>( null );
	let newProjectName  = $state( '' );
	let addedPaths      = $state<string[]>( [] );
	let isAddingProject = $state( false );
	// let errorMessage    = $state( '' );

	// Process Execution & Log State
	let instanceLogs        = $state<Record<string, string>>( {} );
	let runningStatuses     = $state<Record<string, boolean>>( {} );
	let activeLogInstanceId = $state<string | null>( null );

	// Script Manual Addition State
	let pathAddingScript    = $state<string | null>( null );
	let newManualName       = $state( '' );
	let newManualCommand    = $state( '' );

	// Toast Alert State
	let toastMessage    = $state( '' );
	let toastType       = $state<'info' | 'error' | 'success'>( 'info' );

    let toastTimeout: any = null;

	$effect( () => {
		loadProjects();
	});

	// Listeners for Rust events (logs and process status changes)
	$effect( () => {
		let active = true;

        let unlistenLog     : (() => void) | null = null;
		let unlistenStatus  : (() => void) | null = null;

		const setupListeners = async () => {
			const uLog = await listen<{ instance_id: string; text: string }>( 'instance-log', ( ( event ) => {
				if ( !active ) return;

                const { instance_id, text } = event.payload;

                if ( !instanceLogs[ instance_id ] ) {
					instanceLogs[ instance_id ] = '';
				}

                instanceLogs[ instance_id ] += text;

				// Limit logs size to prevent memory leaks
				if ( instanceLogs[ instance_id ].length > 40000 ) {
					instanceLogs[ instance_id ] = instanceLogs[ instance_id ].slice( -20000 );
				}
			}));

			const uStatus = await listen<{
                instance_id: string;
                running: boolean;
                exit_code: number | null
            }>( 'instance-status', ( ( event ) => {
				if ( !active ) return;

                const { instance_id, running } = event.payload;

                runningStatuses[ instance_id ] = running;
			}));

			unlistenLog = uLog;
			unlistenStatus = uStatus;
		};

		setupListeners();

		return () => {
			active = false;
			if ( unlistenLog ) unlistenLog();
			if ( unlistenStatus ) unlistenStatus();
		};
	});

	// Group instances by path reactively using Svelte 5 derived rune
	let groupedInstances = $derived.by( () => {
		if ( !selectedProject ) return {};

        const groups: Record<string, Instance[]> = {};

        for ( const inst of selectedProject.instances ) {
            if ( !groups[ inst.path ] ) {
				groups[ inst.path ] = [];
			}

            groups[ inst.path ].push( inst );
		}

        return groups;
	});


    function showToast( msg: string, type: 'info' | 'error' | 'success' = 'info' ): void {
		toastMessage    = msg;
		toastType       = type;

        if ( toastTimeout ) {
			clearTimeout( toastTimeout );
		}

        toastTimeout = setTimeout(( () => {
			toastMessage = '';
		}), 4000 );
	}


    async function loadProjects() : Promise<void> {
		try {
			projects = await invoke<Project[]>( 'get_projects' );
		} catch ( err ) {
			showToast( String( err ), 'error' );
		}
	}


    async function selectFolderForNewProject() {
		try {
			const path = await invoke<string | null>( 'select_directory' );

            if ( path ) {
				if ( addedPaths.includes( path )) {
					showToast( 'Este directorio ya está en la lista.', 'info' );

                    return;
				}

                addedPaths.push( path );
			}
		} catch ( err ) {
			showToast( String( err ), 'error' );
		}
	}


    function removeAddedPath( index: number ) {
		addedPaths = addedPaths.filter( ( _, i ) => i !== index );
	}


    async function handleAddProject(): Promise<void> {
		if ( !newProjectName ) {
			showToast( 'Por favor, introduce un nombre para el proyecto.', 'error' );
			return;
		}

        if ( addedPaths.length === 0 ) {
			showToast( 'Por favor, añade al menos una ruta al proyecto.', 'error' );
			return;
		}

        try {
			const project = await invoke<Project>( 'add_project', {
				name  : newProjectName,
				paths : addedPaths,
			});

            projects.push( project );

            newProjectName  = '';
			addedPaths      = [];
			isAddingProject = false;

            selectProject( project );
			showToast( 'Proyecto registrado exitosamente.', 'success' );
		} catch ( err ) {
			showToast( String( err ), 'error' );
		}
	}


    async function selectProject( project: Project ): Promise<void> {
		selectedProject = project;
	}


    async function handleDeleteProject( id: string ) : Promise<void> {
		try {
			await invoke( 'delete_project', { id } );

            projects = projects.filter( ( p ) => p.id !== id );

            if ( selectedProject?.id === id ) {
				selectedProject = null;
			}

            showToast( 'Proyecto eliminado.', 'success' );
		} catch ( err ) {
			showToast( String( err ), 'error' );
		}
	}


    async function handleAddPathToSelectedProject() {
		if ( !selectedProject ) return;

        try {
			const path = await invoke<string | null>( 'select_directory' );

            if ( !path ) return;

			const newInstances = await invoke<Instance[]>( 'add_project_path', {
				projectId : selectedProject.id,
				path,
			});

			if ( !selectedProject.paths.includes( path )) {
				selectedProject.paths.push( path );
			}

            selectedProject.instances.push( ...newInstances );

			selectedProject = { ...selectedProject };
			projects        = projects.map( ( p ) => p.id === selectedProject!.id ? selectedProject! : p );

			showToast( `Se agregaron ${ newInstances.length } instancias del nuevo path.`, 'success' );
		} catch ( err ) {
			if ( err === 'ALREADY_REGISTERED' ) {
				showToast( 'Todas las instancias de esta ruta ya están registradas.', 'info' );
			} else {
				showToast( String( err ), 'error' );
			}
		}
	}


    async function handleDeleteInstance( instanceId: string ) {
		if ( !selectedProject ) return;

        try {
			await invoke( 'delete_project_instance', {
				projectId  : selectedProject.id,
				instanceId,
			});

			selectedProject.instances = selectedProject.instances.filter( ( inst ) => inst.id !== instanceId );

            selectedProject = { ...selectedProject };
			projects        = projects.map( ( p ) => p.id === selectedProject!.id ? selectedProject! : p );

			if ( activeLogInstanceId === instanceId ) {
				activeLogInstanceId = null;
			}

			showToast( 'Instancia eliminada del proyecto.', 'success' );
		} catch ( err ) {
			showToast( String( err ), 'error' );
		}
	}

	// Process Controls (START / STOP)
	async function toggleInstance( inst: Instance ) {
		if ( !selectedProject ) return;

        const isRunning = runningStatuses[ inst.id ];

		if ( isRunning ) {
			try {
				await invoke( 'stop_instance', { instanceId : inst.id } );
				showToast( `Deteniendo ${ inst.name }...`, 'info' );
			} catch ( err ) {
				showToast( String( err ), 'error' );
			}
		} else {
			try {
				instanceLogs[ inst.id ] = '';
				activeLogInstanceId = inst.id;

				await invoke( 'start_instance', {
					projectId  : selectedProject.id,
					instanceId : inst.id,
				});

                showToast( `Iniciando ${ inst.name }...`, 'success' );
			} catch ( err ) {
				showToast( String( err ), 'error' );
			}
		}
	}

	// Update Instance Callbacks
	async function handleUpdateInstanceName( inst: Instance, newName: string ) {
		if ( !selectedProject ) return;

        try {
			await invoke( 'update_instance', {
				projectId  : selectedProject.id,
				instanceId : inst.id,
				newName,
				newCommand : inst.command,
			});

			inst.name = newName;
			selectedProject = { ...selectedProject };
			projects = projects.map( ( p ) => p.id === selectedProject!.id ? selectedProject! : p );

            showToast( 'Nombre del script actualizado.', 'success' );
		} catch ( err ) {
			showToast( String( err ), 'error' );
		}
	}


    async function handleUpdateInstanceCommand( inst: Instance, newCommand: string ) {
		if ( !selectedProject ) return;

        try {
			await invoke( 'update_instance', {
				projectId  : selectedProject.id,
				instanceId : inst.id,
				newName    : inst.name,
				newCommand,
			} );

			inst.command = newCommand;
			selectedProject = { ...selectedProject };
			projects = projects.map( ( p ) => p.id === selectedProject!.id ? selectedProject! : p );
			showToast( 'Comando del script actualizado.', 'success' );
		} catch ( err ) {
			showToast( String( err ), 'error' );
		}
	}

	// Inline Manual Addition
	function startAddingManualScript( path: string ) {
		pathAddingScript = path;
		newManualName = '';
		newManualCommand = '';
	}

	function cancelAddingManualScript() {
		pathAddingScript = null;
	}

	function handleManualKeyDown( e: KeyboardEvent, path: string ) {
		if ( e.key === 'Enter' ) {
			e.preventDefault();
			handleSaveManualScript( path );
		} else if ( e.key === 'Escape' ) {
			cancelAddingManualScript();
		}
	}

	async function handleSaveManualScript( path: string ) {
		if ( !selectedProject ) return;
		if ( !newManualName.trim() || !newManualCommand.trim() ) {
			showToast( 'Por favor, llena el nombre y el comando.', 'error' );
			return;
		}

		try {
			const newInst = await invoke<Instance>( 'add_manual_instance', {
				projectId : selectedProject.id,
				path,
				name      : newManualName,
				command   : newManualCommand,
			} );

			selectedProject.instances.push( newInst );
			selectedProject = { ...selectedProject };
			projects = projects.map( ( p ) => p.id === selectedProject!.id ? selectedProject! : p );

			newManualName = '';
			newManualCommand = '';
			pathAddingScript = null;

			showToast( 'Script manual agregado.', 'success' );
		} catch ( err ) {
			showToast( String( err ), 'error' );
		}
	}

	function getActiveInstanceName(): string {
		if ( !activeLogInstanceId || !selectedProject ) return 'Ninguno';
		const inst = selectedProject.instances.find( ( i ) => i.id === activeLogInstanceId );
		return inst ? inst.name : 'Ninguno';
	}

	function handleClearConsole() : void {
		if ( activeLogInstanceId ) {
			instanceLogs[ activeLogInstanceId ] = '';
		}
	}
</script>

{#if logViewerData}
	<LogViewer
		instanceName={ logViewerData.instanceName }
		logs={ logViewerData.logs }
	/>
{:else}
<main class="flex h-screen w-screen overflow-hidden bg-slate-950 text-slate-100 font-sans">

	<!-- Toast Notification -->
	{#if toastMessage}
		<div class="fixed top-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
			<div class="px-5 py-3 rounded-2xl border text-sm font-semibold flex items-center gap-3 shadow-2xl backdrop-blur-md animate-scale-up pointer-events-auto { toastType === 'error' ? 'bg-red-950/80 border-red-500/30 text-red-200' : toastType === 'success' ? 'bg-emerald-950/80 border-emerald-500/30 text-emerald-200' : 'bg-slate-900/90 border-slate-800 text-slate-200' }">
				{#if toastType === 'error'}
					<span class="w-2 h-2 rounded-full bg-red-500"></span>
				{:else if toastType === 'success'}
					<span class="w-2 h-2 rounded-full bg-emerald-500"></span>
				{:else}
					<span class="w-2 h-2 rounded-full bg-violet-500"></span>
				{/if}
				{ toastMessage }
			</div>
		</div>
	{/if}

	<!-- Sidebar -->
	<Sidebar
		{ projects }
		{ selectedProject }
		onSelectProject={ selectProject }
		onDeleteProject={ handleDeleteProject }
		onAddProject={ ( () => isAddingProject = true ) }
	/>

	<!-- Main Panel -->
	<section class="flex-1 flex flex-col min-w-0 relative">
		{#if selectedProject}
			<!-- Project Detail Header -->
			<header class="p-6 border-b border-slate-800 bg-slate-900/50 backdrop-blur-md flex flex-col gap-4">
				<div class="flex items-center justify-between">
					<div class="min-w-0">
						<h2 class="text-2xl font-bold text-white tracking-tight">{ selectedProject.name }</h2>
					</div>
					<div class="flex items-center gap-3">
						<button
							onclick={ handleAddPathToSelectedProject }
							class="px-4 py-1.5 bg-slate-800 hover:bg-slate-700 border border-slate-700 hover:border-slate-600 text-slate-200 hover:text-white rounded-xl text-xs font-bold transition-all flex items-center gap-1.5"
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
							</svg>
							Agregar Path
						</button>
						<span class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 select-none">
							<span class="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></span>
							Conectado
						</span>
					</div>
				</div>

				<!-- Registered Paths List -->
				<div class="space-y-1 bg-slate-950/50 p-3 rounded-2xl border border-slate-800/80">
					<span class="text-[10px] uppercase font-bold text-slate-500 tracking-wider block mb-1.5 select-none">Rutas Vinculadas</span>

                    <div class="grid  grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-3 lg:gap-5">
                        {#each selectedProject.paths as path}
                            <div class="text-xs font-mono text-slate-400 truncate flex items-center gap-2 select-all">
                                <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-slate-600 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                                </svg>
                                { path }
                            </div>
                        {/each}
                    </div>
				</div>
			</header>

			<!-- Project Workspace -->
			<div class="flex-1 overflow-y-auto p-6 space-y-8 bg-slate-950">
				<!-- Grouped Instances Grid -->
				<div class="space-y-8">
					{#each Object.entries( groupedInstances ) as [ path, instances ] ( path )}
						<div class="space-y-4">
							<!-- Group Title -->
							<div class="flex items-center justify-between pb-2 border-b border-slate-800/80 select-none">
								<div class="flex items-center gap-2 min-w-0">
									<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 text-violet-500 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
									</svg>
									<span class="text-xs font-mono text-slate-300 font-bold truncate pr-3">{ path }</span>
									<span class="text-[10px] font-semibold bg-violet-500/10 text-violet-400 px-2 py-0.5 rounded-full border border-violet-500/20">{ instances.length } scripts</span>
								</div>

								<button
									onclick={ ( () => startAddingManualScript( path ) ) }
									class="px-2.5 py-1 hover:bg-slate-800 hover:text-violet-400 rounded-lg text-slate-400 text-xs font-bold transition-all flex items-center gap-1"
								>
									<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
									</svg>
									Script Manual
								</button>
							</div>

							<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-3 lg:gap-5">
								{#each instances as instance ( instance.id )}
									<InstanceCard
										{ instance }
										{ activeLogInstanceId }
										isRunning={ !!runningStatuses[ instance.id ] }
										onSelect={ ( ( id ) => activeLogInstanceId = id ) }
										onDelete={ handleDeleteInstance }
										onToggle={ toggleInstance }
										onUpdateName={ handleUpdateInstanceName }
										onUpdateCommand={ handleUpdateInstanceCommand }
									/>
								{/each}

								<!-- Inline Adding Manual Script Card -->
								{#if pathAddingScript === path}
									<div class="bg-slate-900 border border-violet-500/30 rounded-2xl p-5 shadow-lg flex flex-col justify-between animate-scale-up">
										<div class="space-y-3">
											<div class="flex items-center justify-between select-none">
												<span class="text-[10px] font-bold text-violet-400 uppercase tracking-wider">Nuevo Script Manual</span>
												<button
													onclick={ cancelAddingManualScript }
													class="text-slate-500 hover:text-slate-300 text-sm font-semibold"
												>
													&times;
												</button>
											</div>
											<input
												type="text"
												bind:value={ newManualName }
												onkeydown={ ( ( e ) => handleManualKeyDown( e, path ) ) }
												class="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-violet-500/40 transition-colors"
												placeholder="Nombre (ej: dev:watch)"
											/>
											<input
												type="text"
												bind:value={ newManualCommand }
												onkeydown={ ( ( e ) => handleManualKeyDown( e, path ) ) }
												class="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-violet-500/40 transition-colors"
												placeholder="Comando (ej: pnpm start)"
											/>
										</div>
										<div class="flex items-center justify-end gap-2 pt-3 mt-2 border-t border-slate-800/40 select-none">
											<button
												onclick={ cancelAddingManualScript }
												class="px-2 py-1 text-[10px] font-semibold text-slate-400 hover:text-white hover:bg-slate-800 rounded"
											>
												Cancelar
											</button>
											<button
												onclick={ ( () => handleSaveManualScript( path ) ) }
												class="px-2.5 py-1 bg-violet-600 hover:bg-violet-500 text-[10px] font-semibold text-white rounded shadow"
											>
												Guardar
											</button>
										</div>
									</div>
								{/if}
							</div>
						</div>
					{/each}

					{#if selectedProject.instances.length === 0}
						<div class="bg-slate-900 border border-dashed border-slate-800 rounded-2xl p-8 text-center select-none">
							<p class="text-sm text-slate-500 font-medium">No hay instancias registradas en este proyecto.</p>
						</div>
					{/if}
				</div>

				<!-- Unified Log Terminal -->
				<Console
					{ activeLogInstanceId }
					instanceName={ getActiveInstanceName() }
					logs={ activeLogInstanceId ? ( instanceLogs[ activeLogInstanceId ] || '' ) : '' }
					onClear={ handleClearConsole }
				/>
			</div>
		{:else}
			<!-- Dashboard Landing Empty State -->
			<div class="flex-1 flex flex-col items-center justify-center p-8 bg-linear-to-b from-slate-950 to-slate-900 select-none">
				<div class="w-20 h-20 rounded-3xl bg-linear-to-tr from-violet-500/20 to-indigo-500/20 flex items-center justify-center mb-6 border border-violet-500/10">
					<svg xmlns="http://www.w3.org/2000/svg" class="w-10 h-10 text-violet-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
					</svg>
				</div>
				<h2 class="text-2xl font-bold text-white mb-2">Comienza con DevOrches</h2>
				<p class="text-slate-500 text-sm max-w-sm text-center mb-8 leading-relaxed">
					Agrega un nuevo proyecto local para escanear sus scripts, levantar instancias y controlar tus servicios de desarrollo.
				</p>
				<button
					onclick={ ( () => isAddingProject = true ) }
					class="px-6 py-3 bg-linear-to-r from-violet-600 to-indigo-600 hover:from-violet-500 hover:to-indigo-500 text-white rounded-xl font-bold text-sm shadow-xl shadow-violet-600/10 hover:shadow-violet-600/20 transition-all"
				>
					Registrar Proyecto
				</button>
			</div>
		{/if}
	</section>

	<!-- Add Project Modal Modal -->
	{#if isAddingProject}
		<div class="fixed inset-0 bg-slate-950/80 backdrop-blur-sm flex items-center justify-center p-4 z-50">
			<div class="bg-slate-900 border border-slate-800 rounded-3xl w-full max-w-md p-8 shadow-2xl animate-scale-up">
				<div class="flex items-center justify-between mb-6 select-none">
					<h3 class="text-lg font-bold text-white">Registrar Nuevo Proyecto</h3>
					<button
						onclick={ ( () => { isAddingProject = false; addedPaths = []; } ) }
						class="text-slate-500 hover:text-white transition-colors text-xl font-semibold"
					>
						&times;
					</button>
				</div>

				<form onsubmit={ ( e ) => { e.preventDefault(); handleAddProject(); } } class="space-y-5">
					<div>
						<label for="name" class="block text-xs font-bold text-slate-400 uppercase tracking-widest mb-2 select-none">Nombre del Proyecto</label>
						<input
							type="text"
							id="name"
							bind:value={ newProjectName }
							placeholder="Ej: Workspace Monorepo"
							class="w-full bg-slate-950 border border-slate-800 rounded-xl px-4 py-3 text-sm text-slate-200 placeholder-slate-600 focus:outline-none focus:border-violet-500/50 transition-colors"
						/>
					</div>

					<div>
						<div class="flex items-center justify-between mb-2 select-none">
							<span class="block text-xs font-bold text-slate-400 uppercase tracking-widest">Paths Vinculados</span>
							<button
								type="button"
								onclick={ selectFolderForNewProject }
								class="text-xs font-bold text-violet-400 hover:text-violet-300 transition-colors flex items-center gap-1"
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
								</svg>
								Agregar Carpeta
							</button>
						</div>

						<!-- Added Paths in Form -->
						<div class="space-y-1.5 max-h-36 overflow-y-auto bg-slate-950 p-3 rounded-xl border border-slate-800">
							{#each addedPaths as path, idx}
								<div class="flex items-center justify-between p-2 bg-slate-900 border border-slate-800/80 rounded-lg text-xs font-mono text-slate-300 group">
									<span class="truncate pr-2">{ path }</span>
									<button
										type="button"
										onclick={ ( () => removeAddedPath( idx ) ) }
										class="text-slate-500 hover:text-red-400 p-0.5 rounded transition-colors"
                                        title="Remover Path"
									>
										<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
											<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
										</svg>
									</button>
								</div>
							{/each}

							{#if addedPaths.length === 0}
								<p class="text-xs text-slate-600 italic text-center p-2 select-none">Ningún directorio agregado aún</p>
							{/if}
						</div>
					</div>

					<div class="flex items-center gap-3 pt-3">
						<button
							type="button"
							onclick={ ( () => { isAddingProject = false; addedPaths = []; } ) }
							class="flex-1 py-3 border border-slate-800 hover:bg-slate-800/50 text-slate-400 hover:text-white rounded-xl font-bold text-sm transition-all"
						>
							Cancelar
						</button>
						<button
							type="submit"
							class="flex-1 py-3 bg-linear-to-r from-violet-600 to-indigo-600 hover:from-violet-500 hover:to-indigo-500 text-white rounded-xl font-bold text-sm shadow-xl shadow-violet-600/10 hover:shadow-violet-600/20 transition-all"
						>
							Guardar
						</button>
					</div>
				</form>
			</div>
		</div>
	{/if}
</main>
{/if}

<style>
	@keyframes scaleUp {
		from {
			transform: scale(0.95);
			opacity: 0;
		}
		to {
			transform: scale(1);
			opacity: 1;
		}
	}
	:global(.animate-scale-up) {
		animation: scaleUp 0.2s cubic-bezier(0.16, 1, 0.3, 1) forwards;
	}

	:global( * ) {
		scrollbar-width : thin;
		scrollbar-color : #334155 transparent;
	}

	:global( *::-webkit-scrollbar ) {
		width  : 5px;
		height : 5px;
	}

	:global( *::-webkit-scrollbar-track ) {
		background    : transparent;
		border-radius : 9999px;
	}

	:global( *::-webkit-scrollbar-thumb ) {
		background-color : #334155;
		border-radius    : 9999px;
		border           : 1px solid transparent;
	}

	:global( *::-webkit-scrollbar-thumb:hover ) {
		background-color : #475569;
	}
</style>
