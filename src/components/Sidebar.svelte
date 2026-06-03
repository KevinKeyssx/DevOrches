<script lang="ts">
	import type { Project } from '../lib/types';

	interface Props {
		projects        : Project[];
		selectedProject : Project | null;
		onSelectProject : ( project: Project ) => void;
		onDeleteProject : ( id: string ) => Promise<void>;
		onAddProject    : () => void;
	}


    let {
		projects,
		selectedProject,
		onSelectProject,
		onDeleteProject,
		onAddProject
	}: Props = $props();


    let isCollapsed = $state( false );


    function getInitials( name: string ): string {
		if ( !name ) return '';

        const parts = name.split( ' ' ).filter(( p ) => p.length > 0 );

        if ( parts.length >= 2 ) {
			return ( parts[ 0 ][ 0 ] + parts[ 1 ][ 0 ] ).toUpperCase();
		}

        return name.slice( 0, 2 ).toUpperCase();
	}
</script>


<aside class="relative bg-slate-900 border-r border-slate-800 flex flex-col justify-between select-none transition-all duration-300 ease-in-out { isCollapsed ? 'w-20' : 'w-80' }">
	<!-- Collapse Button (Floating on the right border) -->
	<button
		onclick={ ( () => isCollapsed = !isCollapsed ) }
		class="absolute -right-3 top-6 z-40 w-6 h-6 rounded-full bg-slate-800 border border-slate-700 text-slate-400 hover:text-white flex items-center justify-center shadow-lg transition-all duration-300 hover:scale-110 cursor-pointer"
		title={ isCollapsed ? 'Expandir Sidebar' : 'Colapsar Sidebar' }
		aria-label={ isCollapsed ? 'Expandir Sidebar' : 'Colapsar Sidebar' }
	>
		<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 transform transition-transform duration-300 { isCollapsed ? 'rotate-180' : '' }" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M15 19l-7-7 7-7" />
		</svg>
	</button>

	<div class="overflow-x-hidden transition-all duration-300 { isCollapsed ? 'px-3 py-6' : 'p-6' }">
		<!-- Header / Logo -->
		<div class="flex items-center gap-3 mb-8 { isCollapsed ? 'justify-center' : '' }">
			<div class="rounded-xl bg-linear-to-tr from-violet-500 to-indigo-600 flex items-center justify-center shadow-lg shadow-violet-500/20 shrink-0 transition-all duration-300 { isCollapsed ? 'w-9 h-9' : 'w-10 h-10' }">
				<svg xmlns="http://www.w3.org/2000/svg" class="text-white transition-all duration-300 { isCollapsed ? 'w-5 h-5' : 'w-6 h-6' }" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
				</svg>
			</div>

            {#if !isCollapsed}
				<div class="flex-1 min-w-0 transition-opacity duration-200">
					<h1 class="font-bold text-lg tracking-wider bg-clip-text text-transparent bg-linear-to-r from-violet-400 to-indigo-300">
						DevOrches
					</h1>

                    <span class="text-xs text-slate-500 font-medium">Orquestador Local</span>
				</div>
			{/if}
		</div>

		<!-- Title Section / Add button -->
		<div class="flex items-center mb-5 { isCollapsed ? 'justify-center' : 'justify-between' }">
			{#if !isCollapsed}
				<h2 class="text-xs font-bold text-slate-400 uppercase tracking-widest transition-opacity duration-200">Proyectos</h2>
			{/if}

            <button
				onclick={ onAddProject }
				class="p-1.5 rounded-lg hover:bg-slate-800 text-slate-400 hover:text-white transition-colors"
				title="Registrar Nuevo Proyecto"
			>
				<svg xmlns="http://www.w3.org/2000/svg" class="w-4.5 h-4.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
				</svg>
			</button>
		</div>

		<!-- Projects list with more spaces -->
		<div class="space-y-2.5 overflow-y-auto overflow-x-hidden max-h-[calc(100vh-200px)] { isCollapsed ? 'flex flex-col items-center' : '' }">
			{#each projects as project ( project.id )}
				<div
					class="group flex items-center justify-between rounded-xl transition-all duration-200 cursor-pointer { selectedProject?.id === project.id ? 'bg-violet-600/15 border border-violet-500/40 text-white shadow-sm' : 'border border-transparent text-slate-400 hover:bg-slate-800/60 hover:text-slate-200' } { isCollapsed ? 'p-1' : 'p-3.5' }"
					onclick={ ( () => onSelectProject( project ) ) }
					onkeydown={ ( ( e ) => { if ( e.key === 'Enter' || e.key === ' ' ) { e.preventDefault(); onSelectProject( project ); } } ) }
					role="button"
					tabindex="0"
					title={ project.name }
					aria-label={ project.name }
				>
					{#if isCollapsed}
						<div
							class="w-10 h-10 rounded-xl bg-linear-to-br { selectedProject?.id === project.id ? 'from-violet-500 to-indigo-600 text-white shadow-md' : 'from-slate-800 to-slate-900 border border-slate-700/50 text-slate-400 hover:text-violet-400 hover:border-violet-500/40' } flex items-center justify-center font-bold text-sm tracking-wider transition-all duration-200"
						>
							{ getInitials( project.name ) }
						</div>
					{:else}
						<div class="flex flex-col min-w-0 pr-2">
							<span class="font-semibold truncate text-sm">{ project.name }</span>
							<span class="text-xs text-slate-500 truncate mt-0.5">{ project.paths.length } rutas registradas</span>
						</div>

                        <button
							onclick={ ( ( e ) => { e.stopPropagation(); onDeleteProject( project.id ); } ) }
							class="opacity-0 group-hover:opacity-100 p-1.5 rounded-lg hover:bg-slate-700/80 text-slate-500 hover:text-red-400 transition-all duration-150"
							aria-label="Eliminar proyecto"
							title="Eliminar proyecto"
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
							</svg>
						</button>
					{/if}
				</div>
			{/each}

			{#if projects.length === 0}
				<p class="text-xs text-slate-600 italic p-3 text-center">
					{#if isCollapsed}
						Ø
					{:else}
						No hay proyectos registrados
					{/if}
				</p>
			{/if}
		</div>
	</div>

	<!-- Footer -->
	<div class="p-6 border-t border-slate-800 bg-slate-950/40 text-xs text-slate-600 text-center font-medium">
		{#if isCollapsed}
			v2.0
		{:else}
			v2.0.0 • Hecho con Rust y Svelte 5
		{/if}
	</div>
</aside>
