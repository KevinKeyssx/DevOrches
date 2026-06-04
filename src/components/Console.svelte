<script lang="ts">
	import 'xterm/css/xterm.css';
	import { Terminal }      from 'xterm';
	import { FitAddon }      from 'xterm-addon-fit';
	import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
	import ConsoleActions    from './ConsoleActions.svelte';


	interface Props {
		activeLogInstanceId : string | null;
		instanceName        : string;
		logs                : string;
		onClear?            : () => void;
	}


	let {
		activeLogInstanceId,
		instanceName,
		logs,
		onClear
	}: Props = $props();

	let terminalContainer = $state<HTMLDivElement | null>( null );

	let terminal: Terminal | null = null;
	let fitAddon: FitAddon | null = null;
	let lastWrittenLength = 0;
	let currentInstanceId = '';


	function clearConsole() : void {
		if ( onClear ) {
			onClear();
		} else {
			if ( terminal ) {
				terminal.reset();
				lastWrittenLength = 0;
			}
		}
	}


	function openInNewWindow() : void {
		if ( !logs ) return;

		const key = `log_viewer_${ Date.now() }`;

		localStorage.setItem( key, JSON.stringify( {
			instanceName,
			logs,
		} ) );

		const webview = new WebviewWindow( key, {
			url       : `/?logViewerKey=${ key }`,
			title     : `Logs — ${ instanceName }`,
			width     : 900,
			height    : 640,
			resizable : true,
			center    : true,
			focus     : true,
		} );

		webview.once( 'tauri://error', ( ( e ) => {
			console.error( 'Error al abrir ventana de logs:', e );
			localStorage.removeItem( key );
		} ) );
	}



    $effect( () => {
		if ( !terminalContainer ) return;

		terminal = new Terminal( {
			convertEol  : true,
			cursorBlink : true,
			theme       : {
				background : '#0b0f19', // slate-950
				foreground : '#cbd5e1', // slate-300
				cursor     : '#6366f1', // indigo-500
				black      : '#000000',
				red        : '#ef4444', // red-500
				green      : '#10b981', // emerald-500
				yellow     : '#f59e0b', // amber-500
				blue       : '#3b82f6', // blue-500
				magenta    : '#8b5cf6', // violet-500
				cyan       : '#06b6d4', // cyan-500
				white      : '#f8fafc', // slate-50
			},
			fontSize   : 12,
			fontFamily : 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
		} );

		fitAddon = new FitAddon();
		terminal.loadAddon( fitAddon );
		terminal.open( terminalContainer );

		// Pequeño retardo para asegurar que el contenedor tiene dimensiones renderizadas
		setTimeout( ( () => {
			if ( fitAddon ) {
				fitAddon.fit();
			}
		} ), 50 );

		const handleResize = () => {
			if ( fitAddon ) {
				fitAddon.fit();
			}
		};
		window.addEventListener( 'resize', handleResize );

		return () => {
			window.removeEventListener( 'resize', handleResize );
			if ( terminal ) {
				terminal.dispose();
			}
		};
	} );

	$effect( () => {
		if ( !terminal ) return;

		// Si no hay proceso seleccionado, mostramos el mensaje de bienvenida
		if ( !activeLogInstanceId ) {
			currentInstanceId = '';
			terminal.reset();
			terminal.write( '\x1b[2m// DevOrches v2.0.0 — Esperando selección de proceso...\x1b[0m\r\n' );
			terminal.write( '\x1b[2m// Haz clic en cualquier tarjeta de script para vincular su consola aquí y ver sus logs en tiempo real.\x1b[0m\r\n' );
			lastWrittenLength = 0;
			return;
		}

		// Si cambió la instancia activa, reiniciamos la terminal
		if ( currentInstanceId !== activeLogInstanceId ) {
			currentInstanceId = activeLogInstanceId;
			terminal.reset();
			lastWrittenLength = 0;
		}

		const currentLogs = logs || '';

		if ( currentLogs.length > lastWrittenLength ) {
			const newChunk = currentLogs.slice( lastWrittenLength );
			terminal.write( newChunk );
			lastWrittenLength = currentLogs.length;
		} else if ( currentLogs.length < lastWrittenLength ) {
			terminal.reset();
			terminal.write( currentLogs );
			lastWrittenLength = currentLogs.length;
		}
	} );
</script>

<div class="bg-slate-900 border border-slate-800 rounded-2xl overflow-hidden flex flex-col h-[450px] shadow-xl">
	<div class="px-5 py-3.5 bg-slate-900 border-b border-slate-800 flex items-center justify-between select-none">
		<div class="flex items-center gap-2">
			<span class="w-3 h-3 rounded-full bg-red-500/80"></span>

            <span class="w-3 h-3 rounded-full bg-yellow-500/80"></span>

            <span class="w-3 h-3 rounded-full bg-green-500/80"></span>

            <span class="text-xs text-slate-400 font-semibold ml-2 font-mono">
				<!-- Consola de Logs Unificada: -->
                <span class="text-violet-400 font-bold">{ instanceName }</span>
			</span>
		</div>

        <div class="flex items-center gap-1">
			<ConsoleActions
				logs={ logs }
				instanceName={ instanceName }
				disabled={ !activeLogInstanceId }
				onClear={ clearConsole }
			/>

			<!-- Button 3: Abrir en nueva ventana -->
			<button
				onclick={ openInNewWindow }
				disabled={ !activeLogInstanceId || !logs }
				class="p-1.5 text-slate-400 hover:text-white hover:bg-slate-800/80 rounded-lg transition-all disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:bg-transparent disabled:hover:text-slate-400"
				title="Abrir logs en nueva pestaña"
			>
				<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
				</svg>
			</button>

			<!-- <span class="text-[10px] text-slate-600 font-mono ml-2">terminal</span> -->
		</div>
	</div>

    <div
		bind:this={ terminalContainer }
		class="w-full h-full bg-[#0b0f19] p-4 overflow-hidden"
	></div>
</div>
