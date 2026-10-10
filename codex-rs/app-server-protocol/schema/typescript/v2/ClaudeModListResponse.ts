import type { ModCommand } from "../ModCommand";
import type { ModPane } from "../ModPane";

export type ClaudeModListResponse = { commands: Array<ModCommand>, panes: Array<ModPane>, };
