export interface Project {
	id        : string;
	name      : string;
	paths     : string[];
	instances : Instance[];
	createdAt : string;
	updatedAt : string;
}

export interface Instance {
	id        : string;
	name      : string;
	command   : string;
	cwd       : string | null;
	env       : Record<string, string>;
	path      : string;
	isCustom? : boolean;
}

export interface CustomInstance {
	name        : string;
	path        : string;
	script_name : string;
	command     : string;
}

export interface WorkflowStep {
	name               : string;
	path               : string;
	script_name        : string;
	fail_on_error      : boolean;
	background_delay?  : number;
	env                : Record<string, string> | null;
}

export interface Workflow {
	name      : string;
	instances : CustomInstance[] | null;
	steps     : WorkflowStep[];
}
