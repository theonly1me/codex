import type { ModNode } from "./ModNode";

export type ModElement = { element: string, plugin: string | null, props: Record<string, unknown>, children: Array<ModNode>, };
