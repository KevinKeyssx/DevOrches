<script lang="ts">
	import { Terminal }  from 'xterm';
	import { FitAddon }  from 'xterm-addon-fit';
	import 'xterm/css/xterm.css';
	import ConsoleActions from './ConsoleActions.svelte';

	interface Props {
		instanceName : string;
		logs         : string;
	}

	let {
		instanceName,
		logs
	}: Props = $props();

	let terminalContainer = $state<HTMLDivElement | null>( null );
	let terminal: Terminal | null = null;
	let fitAddon: FitAddon | null = null;

	function clearTerminal() : void {
		if ( terminal ) {
			terminal.reset();
		}
	}

	$effect( () => {
		if ( !terminalContainer ) return;

		terminal = new Terminal( {
			convertEol   : true,
			cursorBlink  : false,
			cursorStyle  : 'bar',
			disableStdin : true,
			scrollback   : 99999,
			theme        : {
				background : '#0b0f19',
				foreground : '#cbd5e1',
				cursor     : '#0b0f19',
				black      : '#000000',
				red        : '#ef4444',
				green      : '#10b981',
				yellow     : '#f59e0b',
				blue       : '#3b82f6',
				magenta    : '#8b5cf6',
				cyan       : '#06b6d4',
				white      : '#f8fafc',
			},
			fontSize   : 12,
			fontFamily : 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
		} );

		fitAddon = new FitAddon();
		terminal.loadAddon( fitAddon );
		terminal.open( terminalContainer );

		setTimeout( ( () => {
			if ( fitAddon ) {
				fitAddon.fit();
			}
			if ( logs && terminal ) {
				terminal.write( logs );
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
</script>

<div class="flex flex-col h-screen w-screen bg-slate-950 text-slate-100 font-sans overflow-hidden">
	<header class="px-5 py-3.5 bg-slate-900 border-b border-slate-800 flex items-center justify-between select-none shrink-0">
		<div class="flex items-center gap-2">
			<span class="w-3 h-3 rounded-full bg-red-500/80"></span>
			<span class="w-3 h-3 rounded-full bg-yellow-500/80"></span>
			<span class="w-3 h-3 rounded-full bg-green-500/80"></span>
			<span class="text-xs text-slate-400 font-semibold ml-2 font-mono">
				Logs — <span class="text-violet-400 font-bold">{ instanceName }</span>
			</span>
		</div>
		<div class="flex items-center gap-2">
			<ConsoleActions
				logs={ logs }
				instanceName={ instanceName }
				onClear={ clearTerminal }
			/>
			<span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[10px] font-semibold bg-violet-500/10 text-violet-400 border border-violet-500/20 select-none">
				<span class="w-1.5 h-1.5 rounded-full bg-violet-500"></span>
				Solo Lectura
			</span>
			<span class="text-[10px] text-slate-600 font-mono">terminal</span>
		</div>
	</header>
	<div
		bind:this={ terminalContainer }
		class="flex-1 bg-[#0b0f19] p-2 overflow-hidden"
	></div>
</div>
