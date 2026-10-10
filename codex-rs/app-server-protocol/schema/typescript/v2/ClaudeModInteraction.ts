
export type ClaudeModInteraction = { "kind": "callback", callbackId: string, value: unknown, } | { "kind": "close", pluginId: string, id: string, } | { "kind": "focus", pluginId: string, id: string, isFocused: boolean, element: string | null, };
