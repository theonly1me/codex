import type { ModCommand } from "./ModCommand";
import type { ModPane } from "./ModPane";

export type ClaudeModEvent = { "kind": "copy", text: string, } | { "kind": "themePicker" } | { "kind": "command", command: ModCommand, } | { "kind": "open", pane: ModPane, } | { "kind": "close", pluginId: string, id: string, } | { "kind": "invalidate", pluginId: string, } | { "kind": "raster", pluginId: string, requestId: string, key: string, cells: string, } | { "kind": "status", pluginId: string, text: string | null, } | { "kind": "toast", pluginId: string, text: string, } | { "kind": "promptFill", pluginId: string, text: string, } | { "kind": "config", key: string, value: unknown, } | { "kind": "error", pluginId: string, message: string, };
