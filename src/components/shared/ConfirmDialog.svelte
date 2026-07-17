<script lang="ts">
	import { Dialog } from 'bits-ui';
	import { fade, scale } from 'svelte/transition';

	interface Props {
		open         : boolean;
		title        : string;
		description  : string;
		confirmText? : string;
		cancelText?  : string;
		variant?     : 'danger' | 'primary' | 'warning';
		onConfirm    : () => void | Promise<void>;
		onCancel?    : () => void;
	}

	let {
		open = $bindable( false ),
		title,
		description,
		confirmText = 'Confirmar',
		cancelText = 'Cancelar',
		variant = 'danger',
		onConfirm,
		onCancel
	}: Props = $props();

	function handleConfirm() : void {
		onConfirm();
		open = false;
	}

	function handleCancel() : void {
		if ( onCancel ) {
			onCancel();
		}
		open = false;
	}
</script>

<Dialog.Root bind:open={ open }>
	<Dialog.Portal>
		<Dialog.Overlay forceMount>
			{#snippet child( { props, open } )}
				{#if open}
					<div
						{ ...props }
						transition:fade={ { duration : 150 } }
						class="fixed inset-0 z-50 bg-slate-950/80 backdrop-blur-xs"
					></div>
				{/if}
			{/snippet}
		</Dialog.Overlay>

		<Dialog.Content forceMount>
			{#snippet child( { props, open } )}
				{#if open}
					<div
						{ ...props }
						transition:scale={ { duration : 150, start : 0.95 } }
						class="fixed left-1/2 top-1/2 z-50 w-full max-w-md -translate-x-1/2 -translate-y-1/2 rounded-2xl border border-slate-800 bg-slate-900 p-6 shadow-2xl"
					>
						<Dialog.Title class="text-lg font-bold text-white">
							{ title }
						</Dialog.Title>

						<Dialog.Description class="text-sm text-slate-400 mt-2">
							{ description }
						</Dialog.Description>

						<div class="mt-6 flex items-center justify-end gap-3">
							<Dialog.Close
								onclick={ handleCancel }
								class="px-4 py-2 text-sm font-semibold rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white transition-all cursor-pointer"
							>
								{ cancelText }
							</Dialog.Close>

							<button
								onclick={ handleConfirm }
								class="px-4 py-2 text-sm font-semibold rounded-xl transition-all cursor-pointer shadow-lg {
									variant === 'danger'
										? 'bg-red-600 hover:bg-red-500 text-white shadow-red-600/20'
										: variant === 'warning'
											? 'bg-amber-600 hover:bg-amber-500 text-white shadow-amber-600/20'
											: 'bg-violet-600 hover:bg-violet-500 text-white shadow-violet-600/20'
								}"
							>
								{ confirmText }
							</button>
						</div>
					</div>
				{/if}
			{/snippet}
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>
