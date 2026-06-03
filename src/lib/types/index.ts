export interface Project {
	id        : string;
	name      : string;
	paths     : string[];
	instances : Instance[];
	createdAt : string;
	updatedAt : string;
}

export interface Instance {
	id      : string;
	name    : string;
	command : string;
	cwd     : string | null;
	env     : Record<string, string>;
	path    : string;
}

export interface WaitCondition {
	port     : number | null;
	logMatch : string | null;
	delayMs  : number | null;
}

export interface WorkflowStep {
	instanceId : string;
	waitFor    : WaitCondition | null;
}

export interface WorkflowPhase {
	name     : string;
	parallel : boolean;
	steps    : WorkflowStep[];
}

export interface Workflow {
	version   : string;
	name      : string | null;
	env       : Record<string, string>;
	instances : Instance[];
	phases    : WorkflowPhase[];
}
