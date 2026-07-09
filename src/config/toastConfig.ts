import toast, { type ToastOptions, type ToastPosition } from 'svelte-french-toast';

export interface CustomToastOptions {
	success  : ToastOptions;
	error    : ToastOptions;
	info     : ToastOptions;
	position : ToastPosition;
}

export const toastOptions: CustomToastOptions = {
	success  : {
		style     : 'background: rgba(2, 24, 18, 0.95); border: 1px solid rgba(16, 185, 129, 0.2); color: #a7f3d0; backdrop-filter: blur(12px); font-family: sans-serif; font-size: 0.875rem; border-radius: 1rem; font-weight: 600; padding: 12px 20px; box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);',
		iconTheme : {
			primary   : '#10b981',
			secondary : 'rgba(2, 24, 18, 0.95)',
		}
	},
	error    : {
		style     : 'background: rgba(24, 2, 2, 0.95); border: 1px solid rgba(239, 68, 68, 0.2); color: #fca5a5; backdrop-filter: blur(12px); font-family: sans-serif; font-size: 0.875rem; border-radius: 1rem; font-weight: 600; padding: 12px 20px; box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);',
		iconTheme : {
			primary   : '#ef4444',
			secondary : 'rgba(24, 2, 2, 0.95)',
		}
	},
	info     : {
		style     : 'background: rgba(8, 14, 28, 0.95); border: 1px solid rgba(59, 130, 246, 0.2); color: #bfdbfe; backdrop-filter: blur(12px); font-family: sans-serif; font-size: 0.875rem; border-radius: 1rem; font-weight: 600; padding: 12px 20px; box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);',
		iconTheme : {
			primary   : '#3b82f6',
			secondary : 'rgba(8, 14, 28, 0.95)',
		}
	},
	position : 'bottom-right'
};

export function showToast( msg: string, type: 'info' | 'error' | 'success' = 'info' ) : void {
	const options: ToastOptions = {
		...toastOptions[ type ],
		position : toastOptions.position
	};

	if ( type === 'success' ) {
		toast.success( msg, options );
	} else if ( type === 'error' ) {
		toast.error( msg, options );
	} else {
		toast( msg, options );
	}
}

