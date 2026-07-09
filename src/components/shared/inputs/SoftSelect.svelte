<script lang="ts">
	import { ChevronDown, Check } from '@lucide/svelte';

	interface Option {
		id   : string;
		name : string;
	}

	interface Props {
		options      : Option[];
		value        : string;
		placeholder? : string;
		label?       : string;
		disabled?    : boolean;
		placement?   : 'top' | 'bottom';
		onchange?    : ( value : string ) => void;
	}

	let {
		options     = [],
		value       = $bindable( '' ),
		placeholder = 'Seleccionar...',
		label,
		disabled    = false,
		placement   = 'bottom',
		onchange,
	} : Props = $props();

	// ─── Reactive States ──────────────────────────────────────────────────────────
	let isOpen    = $state( false );
	let container = $state<HTMLElement | null>( null );

	// ─── Derived: Selected Option Name ───────────────────────────────────────────
	const selectedItem = $derived.by( () : Option | null => {
		return options.find( ( opt : Option ) : boolean => opt.id === value ) || null;
	} );

	// ─── Actions ──────────────────────────────────────────────────────────────────
	function selectOption( id : string ) : void {
		value = id;
		isOpen = false;
		if ( onchange ) {
			onchange( id );
		}
	}

	function handleOutsideClick( event : MouseEvent ) : void {
		if ( isOpen && container && !container.contains( event.target as Node ) ) {
			isOpen = false;
		}
	}
</script>

<svelte:window onclick={ handleOutsideClick } />

<div
	bind:this={ container }
	class="relative w-full text-left select-none text-xs font-semibold { isOpen ? 'z-50' : 'z-0' }"
>
	<!-- Trigger -->
	<button
		type     = "button"
		onclick  = { ( ) : void => { isOpen = !isOpen; } }
		disabled = { disabled }
		class    = "flex h-9 w-full items-center justify-between gap-1.5 rounded-xl border border-slate-800 bg-slate-950/60 px-3 py-1.5 text-slate-200 outline-none transition-all duration-200 { disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer hover:bg-slate-900/60 hover:border-slate-700/80' } { isOpen ? 'ring-2 ring-violet-500/30 border-violet-500/50 shadow-[0_0_12px_rgba(139,92,246,0.1)]' : '' }"
	>
		<span class="truncate text-left text-slate-400">
			{ label || ( selectedItem ? selectedItem.name : placeholder ) }
		</span>
		<ChevronDown class="size-3 text-slate-500 transition-transform duration-200 { isOpen ? 'rotate-180 text-violet-400' : '' }" />
	</button>

	<!-- Dropdown Menu -->
	{#if isOpen}
		<div
			class="absolute z-55 min-w-full w-max max-h-60 overflow-y-auto rounded-xl border border-slate-800 bg-slate-950/95 p-1 shadow-2xl backdrop-blur-xl { placement === 'top' ? 'bottom-full mb-1.5' : 'top-full mt-1' } right-0"
		>
			<div class="flex flex-col gap-0.5">
				{#each options as opt}
					{@const isSelected = opt.id === value}
					<button
						type="button"
						onclick={ () : void => { selectOption( opt.id ); } }
						class="flex w-full items-center justify-between gap-4 px-3 py-2 rounded-lg text-left transition-colors cursor-pointer { isSelected ? 'bg-violet-500/15 text-violet-400 font-bold' : 'text-slate-400 hover:bg-slate-900 hover:text-slate-100' }"
					>
						<span class="truncate">{ opt.name }</span>
						{#if isSelected}
							<Check class="size-3.5 shrink-0 text-violet-400" />
						{/if}
					</button>
				{/each}
			</div>
		</div>
	{/if}
</div>
