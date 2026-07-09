<script lang="ts">
	import { Minus, Plus } from '@lucide/svelte';
	import toast           from 'svelte-french-toast';

	interface InputNumberProps {
		value? : number | string | null;
		min?   : number;
		max?   : number;
		step?  : number;
		width? : string;
		class? : string;
	}

	let {
		value = $bindable( 1 ),
		min,
		max,
		step  = 1,
		width = 'w-7',
		class : containerClass = ''
	} : InputNumberProps = $props();

	function decrease() : void {
		const current = value === '' || value === null || value === undefined ? 0 : Number( value );
		const next    = current - step;
		if ( min === undefined || next >= min ) {
			value = next;
		} else {
			toast.error( `El valor mínimo permitido es ${ min }.` );
			value = min;
		}
	}

	function increase() : void {
		const current = value === '' || value === null || value === undefined ? 0 : Number( value );
		const next    = current + step;
		if ( max === undefined || next <= max ) {
			value = next;
		} else {
			toast.error( `El valor máximo permitido es ${ max }.` );
			value = max;
		}
	}

	function handleChange( event : Event ) : void {
		const input = event.target as HTMLInputElement;

		if ( input.value === '' ) {
			value = '';
			return;
		}

		let numVal = Number( input.value );

		if ( isNaN( numVal ) ) {
			numVal = min !== undefined ? min : 0;
		}

		if ( min !== undefined && numVal < min ) {
			toast.error( `El valor mínimo permitido es ${ min }.` );
			value = min;
			input.value = String( min );
		} else if ( max !== undefined && numVal > max ) {
			toast.error( `El valor máximo permitido es ${ max }.` );
			value = max;
			input.value = String( max );
		} else {
			value = numVal;
		}
	}
</script>

<div class="flex items-stretch rounded-xl border border-slate-800 bg-slate-950/60 overflow-hidden transition-all duration-200 hover:border-slate-700/80 focus-within:ring-2 focus-within:ring-violet-500/30 focus-within:border-violet-500/50 { containerClass }">
	<button
		type       = "button"
		onclick    = { decrease }
		disabled   = { value !== '' && value !== null && value !== undefined && min !== undefined && Number( value ) <= min }
		aria-label = "Disminuir cantidad"
		class      = "flex items-center justify-center px-2.5 text-slate-500 transition-colors hover:bg-violet-500/10 hover:text-violet-400 cursor-pointer select-none disabled:opacity-20 disabled:pointer-events-none"
	>
		<Minus class="h-3 w-3" />
	</button>
	<input
		type     = "number"
		min      = { min }
		max      = { max }
		value    = { value }
		onchange = { handleChange }
		class    = "{ width } border-x border-slate-800 bg-transparent py-1.5 text-center text-[11px] font-bold text-slate-200 focus:outline-none"
	/>
	<button
		type       = "button"
		onclick    = { increase }
		disabled   = { value !== '' && value !== null && value !== undefined && max !== undefined && Number( value ) >= max }
		aria-label = "Aumentar cantidad"
		class      = "flex items-center justify-center px-2.5 text-slate-500 transition-colors hover:bg-violet-500/10 hover:text-violet-400 cursor-pointer select-none disabled:opacity-20 disabled:pointer-events-none"
	>
		<Plus class="h-3 w-3" />
	</button>
</div>

<style>
	input[ type='number' ]::-webkit-outer-spin-button,
	input[ type='number' ]::-webkit-inner-spin-button {
		-webkit-appearance: none;
		margin: 0;
	}
	input[ type='number' ] {
		-moz-appearance: textfield;
		appearance: textfield;
	}
</style>
