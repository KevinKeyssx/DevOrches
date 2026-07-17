<script lang="ts">
	import { untrack } from 'svelte';

    import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';

    import {
        Play,
        Square,
        Plus,
        Trash2,
        ArrowUp,
        ArrowDown,
        Download,
        Upload,
        Save,
        CheckCircle2,
        XCircle,
        Loader2,
        Settings,
        FileText
    } from '@lucide/svelte';

	import type {
        Project,
        Workflow,
        WorkflowStep,
        CustomInstance
    }                   from '../lib/types';
	import SoftSelect   from './shared/inputs/SoftSelect.svelte';
	import Switch       from './shared/inputs/Switch.svelte';
	import InputNumber  from './shared/inputs/InputNumber.svelte';


    interface Props {
		project             : Project;
		runningStatuses     : Record<string, boolean>;
		activeLogInstanceId : string | null;
		onSelectInstance    : ( id: string ) => void;
		showToast           : ( msg: string, type: 'info' | 'error' | 'success' ) => void;
		currentWorkflow     : Workflow | null;
		isWorkflowRunning   : boolean;
		isSidebarCollapsed  : boolean;
	}

	let {
		project,
		runningStatuses,
		activeLogInstanceId,
		onSelectInstance,
		showToast,
		currentWorkflow = $bindable(),
		isWorkflowRunning = $bindable( false ),
		isSidebarCollapsed = true
	}: Props = $props();


	let windowWidth              = $state( typeof window !== 'undefined' ? window.innerWidth : 1280 );
	let isMdViewport             = $derived( windowWidth >= 768 && windowWidth < 1024 );
	let isLgViewport             = $derived( windowWidth >= 1024 );
	let forceMd                  = $derived( !isSidebarCollapsed && windowWidth >= 1280 && windowWidth < 1440 );
	let showTitleDeleteButton    = $derived( forceMd || isMdViewport );
	let showControlsDeleteButton = $derived( !showTitleDeleteButton );


    let workflowName    = $state( 'Flujo Secuencial' );
	let customInstances = $state<CustomInstance[]>( [] );
	let steps           = $state<WorkflowStep[]>( [] );
	let activeStepIndex = $state<number | null>( null );
	let stepStatuses    = $state<string[]>( [] ); // "pending", "running", "success", "failed"

	// Reset active index when workflow stops
	$effect( ( () => {
		if ( !isWorkflowRunning ) {
			activeStepIndex = null;
		}
	}));

	// Synchronize external changes in currentWorkflow (from parent grid edit/delete) into local states
	$effect( () => {
		if ( currentWorkflow ) {
			const wf = currentWorkflow;

            untrack( ( () => {
				if ( workflowName !== wf.name ) {
					workflowName = wf.name;
				}

                if ( JSON.stringify( customInstances ) !== JSON.stringify( wf.instances || [] )) {
					customInstances = wf.instances || [];
				}

                if ( JSON.stringify( steps ) !== JSON.stringify( wf.steps )) {
					steps = wf.steps;

                    // Sync stepEnvLists for any new steps or changes
					steps.forEach( ( step, idx ) => {
						if ( !stepEnvLists[ idx ] ) {
							const envRecord = step.env || {};
							const list      = Object.entries( envRecord ).map( ( [ k, v ] ) => ( { key : k, value : v } ) );

                            list.push( { key : '', value : '' } );

                            stepEnvLists[ idx ]     = list;
							activeEnvTabs[ idx ]    = 'form';
						}
					});
				}
			}));
		}
	});

	// Synchronize local state edits back into currentWorkflow so App.svelte's derived scripts list updates
	$effect( () => {
		currentWorkflow = buildWorkflowObject();
	});

	// States for environment variables editor
	interface EnvPair {
		key   : string;
		value : string;
	}

	let expandedStepIndex   = $state<number | null>( null );
	let activeEnvTabs       = $state<Record<number, 'form' | 'text'>>( {} );
	let stepEnvLists        = $state<Record<number, EnvPair[]>>( {} );
	let stepEnvTexts        = $state<Record<number, string>>( {} );

	// Load workflow on startup or project change
	$effect( ( () => {
		loadWorkflow();
	}));

	// Listeners for Tauri events
	$effect( ( () => {
		let active = true;
		let unlistenStepStatus : ( () => void ) | null = null;

		const setup = async () => {
			const uStep = await listen<{
				step_index : number;
				status     : string;
				exit_code  : number | null;
			}>( 'workflow-step-status', ( ( event ) => {
				if ( !active ) return;

                const { step_index, status } = event.payload;

                stepStatuses[ step_index ] = status;

				if ( status === 'running' ) {
					activeStepIndex = step_index;

                    const step = steps[ step_index ];

                    if ( step ) {
						const rootPath  = project.paths[ 0 ] || '';
						const absPath   = joinPaths( rootPath, step.path );
						const instId    = `${ absPath }#${ step.script_name }`;

                        onSelectInstance( instId );
					}
				}
			}));

			unlistenStepStatus = uStep;
		};

		setup();

		return () => {
			active = false;
	
            if ( unlistenStepStatus ) unlistenStepStatus();
		};
	}));

	// Helpers
	function joinPaths( root: string, rel: string ) : string {
		const r = root.replace( /\\/g, '/' );

        let s = rel.replace( /\\/g, '/' );

        if ( s.startsWith( './' ) ) {
			s = s.substring( 2 );
		}

        if ( r.endsWith( '/' ) ) {
			return `${ r }${ s }`;
		}

        return `${ r }/${ s }`;
	}


	function normalizePath( path: string ) : string {
		if ( !path ) return '';

        let p = path.replace( /\\/g, '/' );

        if ( p.match( /^[A-Za-z]:/ ) ) {
			p = p.charAt( 0 ).toLowerCase() + p.slice( 1 );
		}

        return p;
	}


    function getRelativePath( absPath: string, rootPath: string ) : string {
		const abs  = normalizePath( absPath );
		const root = normalizePath( rootPath );

		if ( abs === root ) {
			return '.';
		}

		if ( abs.startsWith( root )) {
			let rel = abs.substring( root.length );

			if ( rel.startsWith( '/' )) {
				rel = rel.substring( 1 );
			}

			return `./${ rel }`;
		}

		return abs;
	}

	// Dynamic list of all available scripts (regular project scripts + custom instances)
	let availableScripts = $derived.by( () => {
		const list: {
            id          : string;
            name        : string;
            script_name : string;
            path        : string;
            isCustom    : boolean
        }[] = [];

		if ( !project || !project.instances ) {
			return list;
		}

		const rootPath = project.paths && project.paths[ 0 ] ? project.paths[ 0 ] : '';

		// Project instances
		for ( const inst of project.instances ) {
			if ( !inst ) continue;

            list.push( {
				id          : inst.id,
				name        : `${ inst.name } (${ normalizePath( inst.path ) })`,
				script_name : inst.name,
				path        : getRelativePath( inst.path, rootPath ),
				isCustom    : false,
			});
		}

		// Custom instances in workflow
		for ( const ci of customInstances ) {
			if ( !ci ) continue;

            const absPath   = joinPaths( rootPath, ci.path );
			const id        = `${ absPath }#${ ci.script_name }`;

            list.push( {
				id,
				name        : `${ ci.name } [Manual: ${ ci.script_name }]`,
				script_name : ci.script_name,
				path        : ci.path,
				isCustom    : true,
			});
		}

		return list;
	});

	let hasActiveWorkflowProcesses = $derived.by( () => {
		if ( !steps || steps.length === 0 || !project || !project.instances ) return false;

        const rootPath = project.paths && project.paths[ 0 ] ? project.paths[ 0 ] : '';

		function normPath( path: string ) : string {
			if ( !path ) return '';

            let p = path.replace( /\\/g, '/' );

            if ( p.match( /^[A-Za-z]:/ ) ) {
				p = p.charAt( 0 ).toLowerCase() + p.slice( 1 );
			}

            return p;
		}

		function normInstId( id: string ) : string {
			if ( !id ) return '';

            const parts = id.split( '#' );

            if ( parts.length === 2 ) {
				return `${ normPath( parts[ 0 ] ) }#${ parts[ 1 ] }`;
			}

            return normPath( id );
		}

		return steps.some( ( step ) => {
			if ( !step.script_name ) return false;

			// Find project instance
			const relStepPath = step.path;
			const inst = project.instances.find( ( i ) => {
				const relInstPath = getRelativePath( i.path, rootPath );
				return i.name === step.script_name && relInstPath === relStepPath;
			} );

			if ( inst ) {
				const normId = normInstId( inst.id );

                if ( runningStatuses[ normId ] ) {
					return true;
				}
			}

			// Find custom instance
			if ( customInstances ) {
				const ci = customInstances.find( ( c ) => {
					return c.script_name === step.script_name && c.path === relStepPath;
				} );

                if ( ci ) {
					const absPath = joinPaths( rootPath, ci.path );
					const customId = normInstId( `${ absPath }#${ ci.script_name }` );

                    if ( runningStatuses[ customId ] ) {
						return true;
					}
				}
			}

			// Fallback suffix match
			const suffix = `#${ step.script_name }`;

            return Object.entries( runningStatuses ).some( ( [ id, isRunning ] ) => id.endsWith( suffix ) && isRunning );
		} );
	} );

	// Load workflow
	async function loadWorkflow() : Promise<void> {
		try {
			const wf = await invoke<Workflow | null>( 'load_workflow', { projectId : project.id } );

            if ( wf ) {
				workflowName    = wf.name;
				customInstances = wf.instances || [];
				steps           = wf.steps;

				// Prepare env lists
				stepEnvLists = {};
				stepEnvTexts = {};

                steps.forEach( ( step, idx ) => {
					if ( step.background_delay === undefined ) {
						step.background_delay = 0;
					}

					const envRecord = step.env || {};
					const list      = Object.entries( envRecord ).map( ( [ k, v ] ) => ( { key : k, value : v } ) );

                    list.push( { key : '', value : '' } );

                    stepEnvLists[ idx ]     = list;
					activeEnvTabs[ idx ]    = 'form';
				} );
			} else {
				workflowName    = 'Flujo Secuencial';
				customInstances = [];
				steps           = [];
				stepEnvLists    = {};
				stepEnvTexts    = {};
			}

            stepStatuses = steps.map( ( () => 'pending' ) );
		} catch ( err ) {
			showToast( `Error al cargar workflow: ${ err }`, 'error' );
		}
	}

	// Prepare current workflow object to save
	function buildWorkflowObject() : Workflow {
		const cleanSteps = steps.map( ( step, idx ) => {
			const envList = stepEnvLists[ idx ] || [];
			const envRecord : Record<string, string> = {};

            envList.forEach(( p ) => {
				if ( p.key.trim() ) {
					envRecord[ p.key.trim() ] = p.value;
				}
			});

			return {
				name             : step.name,
				path             : step.path,
				script_name      : step.script_name,
				fail_on_error    : step.fail_on_error,
				background_delay : step.background_delay !== undefined ? step.background_delay : 0,
				env              : Object.keys( envRecord ).length > 0 ? envRecord : null,
			};
		});

		return {
			name      : workflowName,
			instances : customInstances.length > 0 ? customInstances : null,
			steps     : cleanSteps,
		};
	}

	// Save workflow
	async function saveWorkflow() : Promise<void> {
		try {
			const wf = buildWorkflowObject();

            await invoke( 'save_workflow', { projectId : project.id, workflow : wf } );

            showToast( 'Workflow guardado en almacenamiento central.', 'success' );
		} catch ( err ) {
			showToast( `Error al guardar workflow: ${ err }`, 'error' );
		}
	}

	// Export workflow
	async function exportWorkflow() : Promise<void> {
		try {
			const wf        = buildWorkflowObject();
			const exported  = await invoke<boolean>( 'export_workflow', { projectId : project.id, workflow : wf } );

            if ( exported ) {
				showToast( 'Workflow exportado exitosamente.', 'success' );
			}
		} catch ( err ) {
			showToast( `Error al exportar workflow: ${ err }`, 'error' );
		}
	}

	// Import workflow
	async function importWorkflow() : Promise<void> {
		try {
			const wf = await invoke<Workflow | null>( 'import_workflow', { projectId : project.id } );

            if ( wf ) {
				showToast( 'Workflow importado exitosamente.', 'success' );
				await loadWorkflow();
			}
		} catch ( err ) {
			showToast( `Error al importar workflow: ${ err }`, 'error' );
		}
	}

	// Run workflow
	async function runWorkflow() : Promise<void> {
		if ( steps.length === 0 ) {
			showToast( 'No hay pasos en el workflow para ejecutar.', 'error' );
			return;
		}

        try {
			const wf = buildWorkflowObject();

            isWorkflowRunning   = true;
			stepStatuses        = steps.map( ( () => 'pending' ));

            await invoke( 'run_workflow', { projectId : project.id, workflow : wf });

            showToast( 'Iniciando workflow...', 'success' );
		} catch ( err ) {
			isWorkflowRunning = false;
			showToast( `Error al iniciar workflow: ${ err }`, 'error' );
		}
	}

	// Abort workflow
	async function abortWorkflow() : Promise<void> {
		try {
			const wf = buildWorkflowObject();

            await invoke( 'abort_workflow', { projectId : project.id, workflow : wf });

            showToast( 'Enviando señal de parada...', 'info' );
		} catch ( err ) {
			showToast( `Error al detener workflow: ${ err }`, 'error' );
		}
	}

	// Add step
	function addStep() : void {
		const newIndex = steps.length;

        steps.push( {
			name             : `Paso ${ newIndex + 1 }`,
			path             : '.',
			script_name      : '',
			fail_on_error    : true,
			background_delay : 0,
			env              : null,
		} );

        stepStatuses.push( 'pending' );

        stepEnvLists[ newIndex ]    = [ { key : '', value : '' } ];
		activeEnvTabs[ newIndex ]   = 'form';
		expandedStepIndex           = newIndex;
	}

	// Remove step
	function removeStep( idx: number ) : void {
		steps = steps.filter( ( _, i ) => i !== idx );
		stepStatuses = stepStatuses.filter( ( _, i ) => i !== idx );

		// Re-align env list records
		const newEnvLists   : Record<number, EnvPair[]>         = {};
		const newEnvTexts   : Record<number, string>            = {};
		const newEnvTabs    : Record<number, 'form' | 'text'>   = {};

		steps.forEach( ( _, i ) => {
			const oldIdx = i >= idx ? i + 1 : i;

            newEnvLists[ i ]    = stepEnvLists[ oldIdx ]    || [ { key : '', value : '' } ];
			newEnvTexts[ i ]    = stepEnvTexts[ oldIdx ]    || '';
			newEnvTabs[ i ]     = activeEnvTabs[ oldIdx ]   || 'form';
		});

		stepEnvLists    = newEnvLists;
		stepEnvTexts    = newEnvTexts;
		activeEnvTabs   = newEnvTabs;

		if ( expandedStepIndex === idx ) {
			expandedStepIndex = null;
		} else if ( expandedStepIndex !== null && expandedStepIndex > idx ) {
			expandedStepIndex--;
		}
	}

	// Reorder steps
	function moveStepUp( idx: number ) : void {
		if ( idx === 0 ) return;

        const temp = steps[ idx ];

        steps[ idx ]        = steps[ idx - 1 ];
		steps[ idx - 1 ]    = temp;

		const tempStatus = stepStatuses[ idx ];

        stepStatuses[ idx ]     = stepStatuses[ idx - 1 ];
		stepStatuses[ idx - 1 ] = tempStatus;

		// Swap env listings
		const list1 = stepEnvLists[ idx ];
		const list2 = stepEnvLists[ idx - 1 ];

        stepEnvLists[ idx ]     = list2;
		stepEnvLists[ idx - 1 ] = list1;

		const text1 = stepEnvTexts[ idx ];
		const text2 = stepEnvTexts[ idx - 1 ];

        stepEnvTexts[ idx ]     = text2;
		stepEnvTexts[ idx - 1 ] = text1;

		const tab1 = activeEnvTabs[ idx ];
		const tab2 = activeEnvTabs[ idx - 1 ];

        activeEnvTabs[ idx ]        = tab2;
		activeEnvTabs[ idx - 1 ]    = tab1;

		if ( expandedStepIndex === idx ) {
			expandedStepIndex = idx - 1;
		} else if ( expandedStepIndex === idx - 1 ) {
			expandedStepIndex = idx;
		}
	}

	function moveStepDown( idx: number ) : void {
		if ( idx === steps.length - 1 ) return;

        moveStepUp( idx + 1 );
	}

	// Dropdown Selection Change
	function handleScriptSelect( idx: number, scriptId: string ) : void {
		const found = ( availableScripts || [] ).find( ( s ) => s.id === scriptId );

        if ( found ) {
			steps[ idx ].script_name = found.script_name;
			steps[ idx ].path = found.path;
			steps[ idx ].name = `Ejecutar ${ found.script_name } en ${ found.path }`;
		}
	}

	// Env Form input changes & enter key handling
	function handleEnvPairChange( stepIdx: number, pairIdx: number ) : void {
		const list = stepEnvLists[ stepIdx ];

        if ( pairIdx === list.length - 1 ) {
			const item = list[ pairIdx ];

            if ( item.key.trim() || item.value.trim() ) {
				// Append new empty row
				list.push( { key : '', value : '' } );
			}
		}
	}

	function handleEnvKeydown( e: KeyboardEvent, stepIdx: number, pairIdx: number ) : void {
		if ( e.key === 'Enter' ) {
			e.preventDefault();
			const list = stepEnvLists[ stepIdx ];

			// Insert a new row after current pair index
			list.splice( pairIdx + 1, 0, { key : '', value : '' } );

			// Focus the newly added input on next tick
			setTimeout( ( () => {
				const inputs = document.querySelectorAll( `.step-${ stepIdx }-env-key` ) as NodeListOf<HTMLInputElement>;

                if ( inputs.length > pairIdx + 1 ) {
					inputs[ pairIdx + 1 ].focus();
				}
			}), 30 );
		}
	}

	function deleteEnvPair( stepIdx: number, pairIdx: number ) : void {
		const list = stepEnvLists[ stepIdx ];

        if ( list.length > 1 ) {
			stepEnvLists[ stepIdx ] = list.filter( ( _, i ) => i !== pairIdx );
		}
	}

	// Tab switching in Env variable editor (Sync)
	function switchEnvTab( stepIdx: number, tab: 'form' | 'text' ) : void {
		activeEnvTabs[ stepIdx ] = tab;

		if ( tab === 'text' ) {
			// Sync Form list -> Textarea
			const list = stepEnvLists[ stepIdx ] || [];
			const pairs = list.filter( ( p ) => p.key.trim() || p.value.trim() );
			stepEnvTexts[ stepIdx ] = pairs.map( ( p ) => `${ p.key }=${ p.value }` ).join( '\n' );
		} else {
			// Sync Textarea -> Form list
			const text = stepEnvTexts[ stepIdx ] || '';
			const parsed = text.split( '\n' ).map( ( line ) => {
				const eqIdx = line.indexOf( '=' );

                if ( eqIdx !== -1 ) {
					return {
						key   : line.substring( 0, eqIdx ).trim(),
						value : line.substring( eqIdx + 1 ).trim(),
					};
				}
				return { key : line.trim(), value : '' };
			}).filter(( p ) => p.key );

			parsed.push( { key : '', value : '' } );
			stepEnvLists[ stepIdx ] = parsed;
		}
	}

	// Custom Instances (Manual scripts in YAML)
	let newCustomName       = $state( '' );
	let newCustomPath       = $state( '' );
	let newCustomScript     = $state( '' );
	let newCustomCommand    = $state( '' );
	let isAddingCustom      = $state( false );

	function addCustomInstance() : void {
		if ( !newCustomName.trim() || !newCustomPath.trim() || !newCustomScript.trim() || !newCustomCommand.trim() ) {
			showToast( 'Por favor rellene todos los campos del script personalizado.', 'error' );

            return;
		}

		customInstances.push({
			name        : newCustomName.trim(),
			path        : newCustomPath.trim().replace( /\\/g, '/' ),
			script_name : newCustomScript.trim(),
			command     : newCustomCommand.trim(),
		});

		newCustomName    = '';
		newCustomPath    = '';
		newCustomScript  = '';
		newCustomCommand = '';
		isAddingCustom   = false;

        showToast( 'Script personalizado agregado al ecosistema.', 'success' );
	}

	function deleteCustomInstance( idx: number ) : void {
		customInstances = customInstances.filter( ( _, i ) => i !== idx );
		showToast( 'Script personalizado eliminado.', 'info' );
	}
</script>

<svelte:window bind:innerWidth={ windowWidth } />

<div class="space-y-6">
	<!-- Top action bar -->
	<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 bg-slate-900/60 p-5 rounded-2xl border border-slate-800/80">
		<div class="flex items-center gap-3 w-full max-w-md">
			<span class="text-xs font-bold text-violet-400 uppercase tracking-widest shrink-0">Nombre:</span>
			<input
				type="text"
				bind:value={ workflowName }
				class="w-full bg-slate-950 border border-slate-800 rounded-xl px-4 py-2 text-sm text-slate-200 focus:outline-none focus:border-violet-500/50 transition-colors font-bold"
				placeholder="Ej: Pipeline de Despliegue"
			/>
		</div>

		<div class="flex items-center gap-2.5 shrink-0 self-end sm:self-auto">
			<button
				onclick={ importWorkflow }
				class="p-2.5 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white rounded-xl border border-slate-700/50 hover:border-slate-600 transition-all"
				title="Importar Workflow (.yml)"
			>
				<Upload class="w-4 h-4" />
			</button>
			<button
				onclick={ exportWorkflow }
				class="p-2.5 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white rounded-xl border border-slate-700/50 hover:border-slate-600 transition-all"
				title="Exportar copia portable (.yml)"
			>
				<Download class="w-4 h-4" />
			</button>
			<button
				onclick={ saveWorkflow }
				class="px-4 py-2.5 bg-slate-800 hover:bg-slate-700 text-slate-200 hover:text-white rounded-xl border border-slate-700 hover:border-slate-600 transition-all text-xs font-bold flex items-center gap-1.5"
			>
				<Save class="w-3.5 h-3.5" />
				Guardar
			</button>

			{#if ( isWorkflowRunning || hasActiveWorkflowProcesses ) }
				<button
					onclick={ abortWorkflow }
					class="px-5 py-2.5 bg-red-600 hover:bg-red-500 text-white rounded-xl text-xs font-bold transition-all flex items-center gap-1.5 shadow-lg shadow-red-600/15"
				>
					<Square class="w-3.5 h-3.5" />
					Abortar Workflow
				</button>
			{:else}
				<button
					onclick={ runWorkflow }
					class="px-5 py-2.5 bg-linear-to-r from-violet-600 to-indigo-600 hover:from-violet-500 hover:to-indigo-500 text-white rounded-xl text-xs font-bold transition-all flex items-center gap-1.5 shadow-lg shadow-violet-600/15"
				>
					<Play class="w-3.5 h-3.5" />
					Ejecutar Workflow
				</button>
			{/if}
		</div>
	</div>

	<!-- Custom YAML-only instances definition -->
	<div class="bg-slate-900/40 p-5 rounded-2xl border border-slate-800/80 space-y-4">
		<div class="flex items-center justify-between gap-2">
			<div>
				<h4 class="text-sm font-bold text-white">Comandos Personalizados (YAML)</h4>
				<p class="text-[11px] text-slate-500">Scripts adicionales portables declarados únicamente en el flujo.</p>
			</div>

            <button
				onclick={ ( () => isAddingCustom = !isAddingCustom ) }
				class="px-3 py-1 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white rounded-lg text-xs font-bold border border-slate-700/50 transition-all flex items-center gap-0"
			>
				<Plus class="w-3.5 h-3.5" />
				Agregar Comando
			</button>
		</div>

		{#if isAddingCustom}
			<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3 bg-slate-950/40 p-4 rounded-xl border border-slate-800/50">
				<div class="space-y-1">
					<label for="newCustomName" class="text-[10px] font-bold text-slate-500 uppercase">Nombre descriptivo</label>
					<input id="newCustomName" type="text" bind:value={ newCustomName } placeholder="Ej: Clean Build" class="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-300 focus:outline-none focus:border-violet-500/40" />
				</div>
				<div class="space-y-1">
					<label for="newCustomPath" class="text-[10px] font-bold text-slate-500 uppercase">Ruta Relativa</label>
					<input id="newCustomPath" type="text" bind:value={ newCustomPath } placeholder="Ej: ./backend" class="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-300 focus:outline-none focus:border-violet-500/40" />
				</div>
				<div class="space-y-1">
					<label for="newCustomScript" class="text-[10px] font-bold text-slate-500 uppercase">Nombre Script (ID)</label>
					<input id="newCustomScript" type="text" bind:value={ newCustomScript } placeholder="Ej: custom:clean" class="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-300 focus:outline-none focus:border-violet-500/40" />
				</div>
				<div class="space-y-1">
					<label for="newCustomCommand" class="text-[10px] font-bold text-slate-500 uppercase">Comando real</label>
					<div class="flex gap-2">
						<input type="text" bind:value={ newCustomCommand } placeholder="Ej: rm -rf dist" class="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-300 focus:outline-none focus:border-violet-500/40" />
						<button onclick={ addCustomInstance } class="px-3 py-1.5 bg-violet-600 hover:bg-violet-500 text-white rounded-lg text-xs font-bold transition-all">Guardar</button>
					</div>
				</div>
			</div>
		{/if}

		{#if customInstances.length > 0}
			<div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
				{#each customInstances as ci, idx}
					<div class="flex items-center justify-between p-3 bg-slate-950/60 border border-slate-800/80 rounded-xl group relative">
						<div class="min-w-0 pr-6">
							<div class="text-xs font-bold text-slate-300 truncate">{ ci.name }</div>
							<div class="text-[10px] font-mono text-slate-500 truncate">{ ci.path } | { ci.script_name }</div>
							<div class="text-[10px] font-mono text-violet-400/80 truncate mt-1 bg-slate-950 p-1 rounded border border-slate-900">{ ci.command }</div>
						</div>
						<button
							onclick={ ( () => deleteCustomInstance( idx ) ) }
							class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 p-1 text-slate-500 hover:text-red-400 rounded hover:bg-slate-800 transition-all"
							title="Eliminar comando"
						>
							<Trash2 class="w-3.5 h-3.5" />
						</button>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Steps sequence vertical list -->
	<div class="space-y-4">
		<div class="flex items-center justify-between border-b border-slate-800 pb-2">
			<h3 class="text-base font-bold text-white">Pasos del Flujo de Trabajo</h3>
			<button
				onclick={ addStep }
				class="px-4 py-1.5 bg-linear-to-r from-violet-600 to-indigo-600 hover:from-violet-500 hover:to-indigo-500 text-white rounded-xl text-xs font-bold transition-all flex items-center gap-1.5 shadow-md shadow-violet-600/10"
			>
				<Plus class="w-4 h-4" />
				Agregar Paso
			</button>
		</div>

		{#if steps.length === 0}
			<div class="bg-slate-900/30 border border-dashed border-slate-800 rounded-2xl p-12 text-center select-none">
				<p class="text-sm text-slate-500 font-medium">Aún no has estructurado ningún paso para este Workflow.</p>
				<button onclick={ addStep} class="mt-4 px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white rounded-xl text-xs font-bold border border-slate-700 transition-all">Crear Primer Paso</button>
			</div>
		{:else}
			<div class="space-y-3.5">
				{#each steps as step, idx ( idx )}
					<div
						class="bg-slate-900 border { activeStepIndex === idx ? 'border-violet-500 ring-1 ring-violet-500/20 shadow-lg shadow-violet-500/5' : 'border-slate-800/80 hover:border-slate-700' } rounded-2xl transition-all duration-200"
					>
						<!-- Step Header Bar -->
						<div class="p-4 flex flex-wrap { ( isLgViewport && !forceMd ) ? 'lg:flex-nowrap' : '' } items-center justify-between gap-4 select-none">
							<div class="flex items-center justify-between { ( isLgViewport && !forceMd ) ? 'lg:justify-start lg:w-auto' : '' } gap-3 w-full flex-1 min-w-[200px]">
								<div class="flex items-center gap-3 min-w-0 flex-1">
									<!-- Execution Status Icon -->
									<div class="shrink-0">
										{#if stepStatuses[ idx ] === 'running'}
											<div class="w-6 h-6 rounded-full bg-violet-500/20 border border-violet-500 flex items-center justify-center">
												<Loader2 class="w-3.5 h-3.5 text-violet-400 animate-spin" />
											</div>
										{:else if stepStatuses[ idx ] === 'success'}
											<div class="w-6 h-6 rounded-full bg-emerald-500/15 flex items-center justify-center">
												<CheckCircle2 class="w-4 h-4 text-emerald-400" />
											</div>
										{:else if stepStatuses[ idx ] === 'failed'}
											<div class="w-6 h-6 rounded-full bg-red-500/15 flex items-center justify-center">
												<XCircle class="w-4 h-4 text-red-400" />
											</div>
										{:else}
											<div class="w-6 h-6 rounded-full bg-slate-950 border border-slate-800 flex items-center justify-center text-[10px] font-bold text-slate-500">
												{ idx + 1 }
											</div>
										{/if}
									</div>

									<div class="min-w-0 flex-1">
										<input
											type="text"
											bind:value={ step.name }
											class="bg-transparent border-0 font-bold text-slate-200 text-sm focus:outline-none focus:bg-slate-950/80 px-2 py-0.5 rounded-lg border-b border-transparent focus:border-slate-800 w-full truncate"
											placeholder="Nombre del paso"
										/>
									</div>
								</div>

								<!-- Delete button for MD breakpoint ONLY (>= md and < lg) -->
								{#if showTitleDeleteButton}
									<button
										onclick={ ( () => removeStep( idx ) ) }
										class="flex p-2 text-slate-500 hover:text-red-400 rounded-xl hover:bg-slate-800 transition-all shrink-0"
										title="Eliminar paso"
									>
										<Trash2 class="w-4 h-4" />
									</button>
								{/if}
							</div>

							<!-- Controls -->
							<div class="flex flex-col md:flex-row items-stretch md:items-center gap-3 w-full { ( isLgViewport && !forceMd ) ? 'lg:w-auto' : '' } min-w-0">
								<!-- Dropdown Selector of existing scripts -->
								<div class="w-full md:w-[200px] shrink-0">
									<SoftSelect
										options={ availableScripts }
										value={ ( availableScripts || [] ).find( ( s ) => s.script_name === step.script_name && s.path === step.path )?.id || '' }
										onchange={ ( val ) => handleScriptSelect( idx, val ) }
										placeholder="Selecciona Script..."
									/>
								</div>

								<!-- Inputs & Buttons Row Container -->
								<div class="flex flex-col sm:flex-row sm:flex-nowrap items-stretch sm:items-center gap-3 w-full md:w-auto shrink-0">
									<!-- Checkbox fail on error & background delay wrapper -->
									<div class="flex flex-row flex-nowrap items-center gap-2 w-full sm:w-auto shrink-0">
										<div class="flex items-center gap-2 bg-slate-950/40 border border-slate-800/80 px-3 py-1.5 rounded-xl hover:bg-slate-950/60 transition-colors w-full sm:w-auto justify-between sm:justify-start">
											<Switch bind:checked={ step.fail_on_error } showLabelText={ false } />
											<span class="text-[10px] font-bold text-slate-400 uppercase tracking-wider select-none whitespace-nowrap">Detener si falla</span>
										</div>

										<!-- Number input for background delay (only shown if not the last step) -->
										{#if ( idx < steps.length - 1 ) }
											<div class="flex items-center gap-2 bg-slate-950/40 border border-slate-800/80 pl-3 pr-1 py-1 rounded-xl w-full sm:w-auto justify-between sm:justify-start">
												<span class="text-[10px] font-bold text-slate-400 uppercase tracking-wider select-none whitespace-nowrap">Espera (s):</span>
												<InputNumber
													bind:value={ step.background_delay }
													min={ 0 }
													max={ 300 }
													width="w-10"
												/>
											</div>
										{/if}
									</div>

									<!-- Settings/Env variables button & action buttons wrapper -->
									<div class="flex items-center justify-between sm:justify-start gap-2 w-full sm:w-auto shrink-0">
										<div class="flex items-center gap-1">
											<button
												onclick={ ( () => expandedStepIndex = expandedStepIndex === idx ? null : idx ) }
												class="p-2 text-slate-400 hover:text-violet-400 hover:bg-slate-800 rounded-xl transition-all"
												title="Configurar Variables de Entorno"
											>
												<Settings class="w-4 h-4 { expandedStepIndex === idx ? 'text-violet-400 rotate-45' : '' } transition-transform duration-200" />
											</button>

											<!-- Order arrows -->
											<button onclick={ ( () => moveStepUp( idx ) ) } disabled={ idx === 0 } class="p-1.5 text-slate-500 hover:text-slate-200 disabled:opacity-30 rounded hover:bg-slate-800">
												<ArrowUp class="w-3.5 h-3.5" />
											</button>
											<button onclick={ ( () => moveStepDown( idx ) ) } disabled={ idx === steps.length - 1 } class="p-1.5 text-slate-500 hover:text-slate-200 disabled:opacity-30 rounded hover:bg-slate-800">
												<ArrowDown class="w-3.5 h-3.5" />
											</button>
										</div>

										<!-- Delete button -->
										{#if showControlsDeleteButton}
											<button
												onclick={ ( () => removeStep( idx ) ) }
												class="p-2 text-slate-500 hover:text-red-400 rounded-xl hover:bg-slate-800 transition-all"
												title="Eliminar paso"
											>
												<Trash2 class="w-4 h-4" />
											</button>
										{/if}
									</div>
								</div>
							</div>
						</div>

						<!-- Step Environment Variables editor panel (Collapsible) -->
						{#if expandedStepIndex === idx}
							<div class="border-t border-slate-800/80 p-5 bg-slate-950/30 rounded-b-2xl space-y-4">
								<div class="flex items-center justify-between border-b border-slate-800/60 pb-2">
									<span class="text-[10px] font-bold text-slate-400 uppercase tracking-widest">Variables de Entorno (env)</span>

									<!-- Editor tabs selector -->
									<div class="flex bg-slate-950 border border-slate-800 p-0.5 rounded-lg">
										<button
											onclick={ ( () => switchEnvTab( idx, 'form' ) ) }
											class="px-2.5 py-1 text-[10px] font-bold rounded-md transition-all flex items-center gap-1 { activeEnvTabs[ idx ] === 'form' ? 'bg-slate-850 text-white' : 'text-slate-500 hover:text-slate-300' }"
										>
											<Settings class="w-3 h-3" />
											Formulario
										</button>
										<button
											onclick={ ( () => switchEnvTab( idx, 'text' ) ) }
											class="px-2.5 py-1 text-[10px] font-bold rounded-md transition-all flex items-center gap-1 { activeEnvTabs[ idx ] === 'text' ? 'bg-slate-850 text-white' : 'text-slate-500 hover:text-slate-300' }"
										>
											<FileText class="w-3 h-3" />
											Plano (.env)
										</button>
									</div>
								</div>

								{#if activeEnvTabs[ idx ] === 'form'}
									<!-- Key-Value editor form view -->
									<div class="space-y-2 max-h-56 overflow-y-auto pr-2">
										{#each stepEnvLists[ idx ] || [] as pair, pIdx ( pIdx )}
											<div class="flex items-center gap-2 animate-scale-up">
												<input
													type="text"
													bind:value={ pair.key }
													oninput={ ( () => handleEnvPairChange( idx, pIdx ) ) }
													onkeydown={ ( ( e ) => handleEnvKeydown( e, idx, pIdx ) ) }
													class="step-{ idx }-env-key w-1/3 bg-slate-950 border border-slate-850 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-violet-500/30 font-mono"
													placeholder="CLAVE (ej: PORT)"
												/>
												<span class="text-slate-600 font-mono">=</span>
												<input
													type="text"
													bind:value={ pair.value }
													oninput={ ( () => handleEnvPairChange( idx, pIdx ) ) }
													onkeydown={ ( ( e ) => handleEnvKeydown( e, idx, pIdx ) ) }
													class="w-2/3 bg-slate-950 border border-slate-850 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-violet-500/30 font-mono"
													placeholder="Valor (ej: 3000)"
												/>
												{#if pIdx < ( stepEnvLists[ idx ] || [] ).length - 1}
													<button
														onclick={ ( () => deleteEnvPair( idx, pIdx ) ) }
														class="p-1.5 text-slate-650 hover:text-red-400 hover:bg-slate-900 rounded-lg transition-colors"
														title="Quitar variable"
													>
														<Trash2 class="w-3.5 h-3.5" />
													</button>
												{:else}
													<div class="w-8 h-8"></div>
												{/if}
											</div>
										{/each}
									</div>
									<p class="text-[10px] text-slate-600 italic">Tip: Presiona Enter en cualquier campo para insertar una nueva variable de entorno.</p>
								{:else}
									<!-- Plaintext .env textarea view -->
									<textarea
										bind:value={ stepEnvTexts[ idx ] }
										oninput={ ( () => switchEnvTab( idx, 'text' ) ) }
										class="w-full h-36 bg-slate-950 border border-slate-850 rounded-xl p-3.5 text-xs text-slate-200 focus:outline-none focus:border-violet-500/30 font-mono"
										placeholder="PORT=3000&#10;API_URL=http://localhost:4000"
									></textarea>
									<p class="text-[10px] text-slate-650 italic">Tip: Copia y pega variables en formato CLAVE=VALOR normales. Cada línea corresponde a una variable.</p>
								{/if}
							</div>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>
