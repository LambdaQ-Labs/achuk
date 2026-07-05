/* tslint:disable */
/* eslint-disable */

/**
 * Type-directed search: which scope symbols unify with this signature?
 * The same freshen+unify query the native CDB runs.
 */
export function candidates(sig: string): string;

/**
 * Check a Def-JSON array against the scope: hallucinated references and
 * effect-row soundness (name-based, mirroring the native grader).
 */
export function check_defs(json: string): string;

/**
 * The decode grammar for the current scope — the actual GBNF projection
 * llama.cpp consumes, from the actual claw-constraint crate.
 */
export function grammar(): string;

/**
 * Parse and echo a type signature — the "is this a valid type?" probe.
 */
export function parse_sig(src: string): string;

/**
 * Render a Def-JSON array as .claw source.
 */
export function render(json: string): string;

/**
 * Run `name(args…)` over a Def-JSON array with the REAL step-bounded
 * interpreter. Args: JSON array of ints/strings/bools.
 */
export function run(defs_json: string, name: string, args_json: string): string;

/**
 * Load the scope: a JSON array of { name, ty, effects? }.
 */
export function set_scope(json: string): number;

/**
 * Every symbol in scope, `name : type` per line.
 */
export function symbols(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly candidates: (a: number, b: number) => [number, number, number, number];
    readonly check_defs: (a: number, b: number) => [number, number, number, number];
    readonly grammar: () => [number, number];
    readonly parse_sig: (a: number, b: number) => [number, number, number, number];
    readonly render: (a: number, b: number) => [number, number, number, number];
    readonly run: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly set_scope: (a: number, b: number) => [number, number, number];
    readonly symbols: () => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
