<script lang="ts">
	import { Terminal } from 'xterm';
	import { FitAddon } from 'xterm-addon-fit';
	import 'xterm/css/xterm.css';

	interface Props {
		activeLogInstanceId : string | null;
		instanceName        : string;
		logs                : string;
	}

	let {
		activeLogInstanceId,
		instanceName,
		logs
	}: Props = $props();

	let terminalContainer = $state<HTMLDivElement | null>( null );
	let terminal: Terminal | null = null;
	let fitAddon: FitAddon | null = null;

	let lastWrittenLength = 0;
	let currentInstanceId = '';

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
				Consola de Logs Unificada: <span class="text-violet-400 font-bold">{ instanceName }</span>
			</span>
		</div>
		<div class="text-[10px] text-slate-600 font-mono">terminal</div>
	</div>
	<div
		bind:this={ terminalContainer }
		class="flex-1 bg-slate-950 p-4 overflow-hidden"
	></div>
</div>
