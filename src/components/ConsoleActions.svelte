<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';

	interface Props {
		logs         : string;
		instanceName : string;
		disabled?    : boolean;
		onClear      : () => void;
	}

	let {
		logs,
		instanceName,
		disabled = false,
		onClear
	}: Props = $props();

	let copySuccess = $state( false );

	async function downloadLogs() : Promise<void> {
		if ( !logs ) return;

		const defaultName = `${ instanceName || 'console' }_logs.log`;

		try {
			await invoke<boolean>( 'save_log_file', {
				defaultName : defaultName,
				content     : logs,
			} );
		} catch ( err ) {
			console.error( 'Error al guardar el archivo de log:', err );
		}
	}

	function copyLogs() : void {
		if ( !logs ) return;

		navigator.clipboard.writeText( logs ).then( ( () => {
			copySuccess = true;

			setTimeout( ( () => {
				copySuccess = false;
			} ), 2000 );
		} ) ).catch( ( ( err ) => {
			console.error( 'Error al copiar al portapapeles:', err );
		} ) );
	}
</script>

<div class="flex items-center gap-1">
	<!-- Button 1: Limpiar consola -->
	<button
		onclick={ onClear }
		disabled={ disabled }
		class="p-1.5 text-slate-400 hover:text-white hover:bg-slate-800/80 rounded-lg transition-all disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:bg-transparent disabled:hover:text-slate-400"
		title="Limpiar consola"
	>
		<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
		</svg>
	</button>

	<!-- Button 2: Descargar log -->
	<button
		onclick={ downloadLogs }
		disabled={ disabled || !logs }
		class="p-1.5 text-slate-400 hover:text-white hover:bg-slate-800/80 rounded-lg transition-all disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:bg-transparent disabled:hover:text-slate-400"
		title="Descargar logs (.log)"
	>
		<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
		</svg>
	</button>

	<!-- Button 3: Copiar clipboard -->
	<button
		onclick={ copyLogs }
		disabled={ disabled || !logs }
		class="p-1.5 rounded-lg transition-all disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:bg-transparent { copySuccess ? 'text-emerald-400 bg-emerald-500/10 hover:bg-emerald-500/20' : 'text-slate-400 hover:text-white hover:bg-slate-800/80' }"
		title={ copySuccess ? "¡Copiado!" : "Copiar logs al portapapeles" }
	>
		{#if copySuccess}
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
			</svg>
		{:else}
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
			</svg>
		{/if}
	</button>
</div>
