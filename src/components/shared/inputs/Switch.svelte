<script lang="ts">
	interface Props {
		checked?       : boolean;
		id?            : string;
		labelActive?   : string;
		labelInactive? : string;
		showLabelText? : boolean;
		class?         : string;
	}

	let {
		checked       = $bindable( false ),
		id            = `switch-${ Math.random().toString( 36 ).substring( 2, 9 ) }`,
		labelActive   = 'Habilitado',
		labelInactive = 'Deshabilitado',
		showLabelText = true,
		class : className = '',
	} : Props = $props();
</script>

<label
	class = "flex cursor-pointer select-none items-center gap-2.5 { className }"
	for   = { id }
>
    <!-- Label text -->
    {#if ( showLabelText )}
        <span
            class = "text-[0.72rem] font-bold tracking-wide transition-colors duration-200 { checked ? 'text-violet-400' : 'text-slate-500' }"
        >
            { checked ? labelActive : labelInactive }
        </span>
    {/if}

    <span
		class = "relative inline-flex h-5 w-9 shrink-0 rounded-full border transition-all duration-200 { checked ? 'bg-violet-600/30 border-violet-500' : 'bg-slate-950 border-slate-800' }"
	>
		<input
			id           = { id }
			type         = "checkbox"
			bind:checked={ checked }
			class        = "sr-only"
		/>
		<!-- Thumb -->
		<span
			class = "absolute top-0.5 left-0.5 h-3.5 w-3.5 rounded-full transition-all duration-200 { checked ? 'translate-x-4 bg-violet-400 shadow-[0_0_10px_rgba(139,92,246,0.5)]' : 'bg-slate-500 translate-x-0' }"
		></span>
	</span>
</label>
