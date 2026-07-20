<script lang="ts">
    import { Play, Square, Cpu, MemoryStick } from '@lucide/svelte';

    import type { Instance } from '../lib/types';


    interface Props {
		instance            : Instance;
		activeLogInstanceId : string | null;
		isRunning           : boolean;
		isWorkflowRunning   : boolean;
		stats               : {
			cpu          : number;
			memory       : number;
			total_memory : number;
		} | undefined;
		onSelect            : ( id: string ) => void;
		onDelete            : ( id: string ) => Promise<void>;
		onToggle            : ( inst: Instance ) => Promise<void>;
		onUpdateName        : ( inst: Instance, newName: string ) => Promise<void>;
		onUpdateCommand     : ( inst: Instance, newCommand: string ) => Promise<void>;
	}


	let {
		instance,
		activeLogInstanceId,
		isRunning,
		isWorkflowRunning,
		stats,
		onSelect,
		onDelete,
		onToggle,
		onUpdateName,
		onUpdateCommand
	}: Props = $props();


	function formatMemory( bytes: number ) : string {
		const mb = bytes / ( 1024 * 1024 );
		return `${ mb.toFixed( 0 ) } MB`;
	}


    function formatMemoryPercentage( bytes: number, totalBytes: number ) : string {
		const percentage = totalBytes > 0 ? ( bytes / totalBytes ) * 100 : 0;
		return `${ percentage.toFixed( 1 ) }%`;
	}


    let isEditingName       = $state( false );
	let isEditingCommand    = $state( false );
	let editName            = $state( '' );
	let editCommand         = $state( '' );


    function startNameEditing() {
		isEditingName   = true;
		editName        = instance.name;
	}


    function cancelNameEditing() {
		isEditingName = false;
	}


    async function saveNameEditing() {
		if ( !editName.trim() ) return;

        if ( editName.trim( ) === instance.name ) {
            isEditingName = false;
            return;
        }

        await onUpdateName( instance, editName.trim() );

        isEditingName = false;
	}


    function startCommandEditing() {
		isEditingCommand = true;
		editCommand = instance.command;
	}


    function cancelCommandEditing() {
		isEditingCommand = false;
	}


    async function saveCommandEditing() {
		if ( !editCommand.trim() ) return;

        if ( editCommand.trim( ) === instance.command ) {
            isEditingCommand = false;
            return;
        }

		await onUpdateCommand( instance, editCommand.trim() );
		isEditingCommand = false;
	}

	function autofocus( node: HTMLInputElement ) {
		node.focus();
	}
</script>


<div
	class       = "{ instance.isCustom ? 'bg-slate-800' : 'bg-slate-900' } border { activeLogInstanceId === instance.id ? 'border-violet-500/50 ring-1 ring-violet-500/20' : 'border-slate-800/80' } rounded-2xl p-5 hover:border-slate-700/80 transition-all duration-300 shadow-md flex flex-col justify-between group relative cursor-pointer"
	onclick     = { ( () => onSelect( instance.id ) ) }
	onkeydown   = { ( ( e ) => { if ( e.key === 'Enter' || e.key === ' ' ) { e.preventDefault(); onSelect( instance.id ); } } ) }
	role        = "button"
	tabindex    = "0"
>
	<!-- Delete Button -->
	{#if !isRunning && !isWorkflowRunning}
		<button
			onclick={ ( ( e ) => { e.stopPropagation(); onDelete( instance.id ); } ) }
			class="absolute top-4 right-4 opacity-0 group-hover:opacity-100 p-1.5 rounded-lg hover:bg-slate-800 text-slate-500 hover:text-red-400 transition-all duration-150"
			title="Eliminar Instancia"
		>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
			</svg>
		</button>
	{/if}

	<div>
		<div class="flex items-center justify-between mb-3 pr-6 select-none">
			<div class="flex items-center gap-1.5 min-w-0 w-full">
				{#if isEditingName}
					<div class="flex items-center gap-1.5 w-full mr-2">
						<input
							use:autofocus
							type="text"
							bind:value={ editName }
							onclick={ ( ( e ) => e.stopPropagation() ) }
							onkeydown={ ( ( e ) => { e.stopPropagation(); if ( e.key === 'Enter' ) { saveNameEditing(); } if ( e.key === 'Escape' ) { cancelNameEditing(); } } ) }
							class="w-full max-w-[180px] bg-slate-950 border border-slate-800 rounded-lg px-2.5 py-0.5 text-base font-bold text-slate-200 focus:outline-none focus:border-violet-500/50 transition-colors"
							placeholder="Nombre"
						/>

                        <button
							onclick={ ( ( e ) => { e.stopPropagation(); saveNameEditing(); } ) }
							class="p-1 rounded-lg text-emerald-400 hover:text-emerald-300 hover:bg-slate-800 transition-all shrink-0"
							title="Guardar nombre"
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
							</svg>
						</button>

                        <button
							onclick = { ( ( e ) => { e.stopPropagation(); cancelNameEditing(); } ) }
							class   = "p-1 rounded-lg text-red-400 hover:text-red-300 hover:bg-slate-800 transition-all shrink-0"
							title   = "Cancelar"
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4.5 h-4.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
							</svg>
						</button>
					</div>
				{:else}
					<span class="font-bold text-slate-200 text-base truncate">{ instance.name }</span>
					{#if instance.isCustom}
						<span class="w-1.5 h-1.5 rounded-full bg-blue-500 animate-pulse shrink-0" title="Workflow Custom Command"></span>
					{/if}

                    {#if !isRunning && !isWorkflowRunning}
						<button
							onclick = { ( ( e ) => { e.stopPropagation(); startNameEditing(); } ) }
							class   = "p-1 rounded-md text-slate-500 hover:text-violet-400 hover:bg-slate-800 transition-all"
							title   = "Editar nombre del script"
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
							</svg>
						</button>
					{/if}
				{/if}
			</div>

            <!-- <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-slate-800/80 text-slate-500 border border-slate-700/30 shrink-0">script</span> -->
            <div class="flex items-center gap-3 text-xs text-slate-500 mr-2">
                <span class="flex items-center gap-1">
                    <span class="w-1.5 h-1.5 rounded-full { isRunning ? 'bg-emerald-500 animate-pulse' : 'bg-slate-700' }"></span>

                    { isRunning ? 'Activo' : 'Inactivo' }
                </span>
            </div>
		</div>

		<div class="relative group/cmd">
			{#if isEditingCommand}
				<div class="flex items-center gap-1.5 w-full mb-4">
					<input
						use:autofocus
						type="text"
						bind:value={ editCommand }
						onclick={ ( ( e ) => e.stopPropagation() ) }
						onkeydown={ ( ( e ) => { e.stopPropagation(); if ( e.key === 'Enter' ) { saveCommandEditing(); } if ( e.key === 'Escape' ) { cancelCommandEditing(); } } ) }
						class="w-full bg-slate-950 border border-slate-800 rounded-lg px-2.5 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-violet-500/40 transition-colors font-mono"
						placeholder="Comando"
					/>

                    <button
						onclick={ ( ( e ) => { e.stopPropagation(); saveCommandEditing(); } ) }
						class="p-1.5 rounded-lg text-emerald-500 hover:text-emerald-400 hover:bg-slate-800 transition-all shrink-0"
						title="Guardar comando"
					>
						<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
						</svg>
					</button>

                    <button
						onclick={ ( ( e ) => { e.stopPropagation(); cancelCommandEditing(); } ) }
						class="p-1.5 rounded-lg text-slate-500 hover:text-red-400 hover:bg-slate-800 transition-all shrink-0"
						title="Cancelar"
					>
						<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
						</svg>
					</button>
				</div>
			{:else}
				<p class="text-xs font-mono text-slate-500 truncate mb-4 select-all bg-slate-950/40 p-2 rounded-lg border border-slate-950 pr-8">
					{ instance.command }
				</p>

				{#if !isRunning && !isWorkflowRunning}
					<button
						onclick={ ( ( e ) => { e.stopPropagation(); startCommandEditing(); } ) }
						class="absolute right-2 top-1.5 opacity-0 group-hover/cmd:opacity-100 p-1 rounded-md text-slate-500 hover:text-violet-400 hover:bg-slate-800/60 transition-all"
						title="Editar comando"
					>
						<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
						</svg>
					</button>
				{/if}
			{/if}
		</div>
	</div>

	<div class="flex items-center justify-between pt-2 border-t border-slate-800/60 mt-2 select-none">
		{#if isRunning && stats}
			<div class="grid sm:flex items-center gap-2 bg-slate-950/60 backdrop-blur-xs px-3.5 py-1.5 rounded-xl border border-slate-800/50 hover:border-violet-500/20 transition-colors duration-300 shadow-inner">
				<div class="flex items-center gap-1.5">
					<!-- CPU Icon -->
                    <Cpu class="size-3.5 text-violet-400" />

                    <span class="text-[10px] font-bold text-slate-400 tracking-wider md:hidden 2xl:flex">CPU:</span>

                    <span class="text-xs font-bold font-mono text-emerald-400 animate-pulse">{ stats.cpu.toFixed( 1 ) }%</span>
				</div>
				<!-- <div class="w-px h-3 bg-slate-800/60 hidden md:flex"></div> -->
				<div class="flex items-center gap-1.5">
					<!-- RAM Icon -->
                    <MemoryStick class="size-3.5 text-violet-400" />

                    <span class="text-[10px] font-bold text-slate-400 tracking-wider md:hidden 2xl:flex">RAM:</span>

                    <span class="text-[11px] font-bold font-mono text-violet-300">{ formatMemory( stats.memory ) }</span>

                    <span class="text-[11px] font-bold font-mono text-violet-300 md:hidden 2xl:flex">{ formatMemoryPercentage( stats.memory, stats.total_memory ) }</span>
				</div>
			</div>
		{:else}
			<div class="text-[11px] font-mono text-slate-500 italic">
				Inactivo
			</div>
		{/if}

        <button
			disabled={ isEditingName || isEditingCommand }
			onclick={ ( ( e ) => { e.stopPropagation(); onToggle( instance ); } ) }
			class="px-3 py-1.5 {
                isRunning
                ? 'bg-red-600 hover:bg-red-500 shadow-red-600/10'
                : 'bg-linear-to-r from-violet-600 to-indigo-600 hover:from-violet-500 hover:to-indigo-500 shadow-violet-600/10'
            } text-white text-xs font-bold rounded-xl transition-all shadow-lg hover:shadow-xl disabled:opacity-30 disabled:pointer-events-none"
		>
            {#if isRunning}
                <Square class="size-4" />
            {:else}
                <Play class="size-4"/>
            {/if}
		</button>
	</div>
</div>
