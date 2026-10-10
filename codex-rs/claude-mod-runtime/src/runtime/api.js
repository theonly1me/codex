import { signalFor } from "codex:mods/cancellation";
import { clockFor } from "codex:mods/clock";
import { uiFor } from "codex:mods/ui";

const methods = {
  fs: ["read", "write", "list", "exists", "stat", "ancestors"],
  env: ["get"],
  store: ["get", "set", "delete", "keys"],
  state: ["get", "set"],
  process: ["run"],
  model: ["complete", "fork"],
  command: ["register", "run", "list"],
  prompt: ["submit", "fill", "read", "compose"],
  session: ["append", "cwd", "root", "id", "usage", "model", "messages", "surfaces", "version", "repo", "turns"],
  config: ["list", "set"],
  settings: ["read"],
};

export function engineFor(plugin, dispatchId) {
  const signal = signalFor(dispatchId);
  const call = (method, argumentsList) => { signal.throwIfAborted(); return __modsHost({ plugin: plugin.id, dispatchId, method, arguments: argumentsList }); };
  const engine = {};
  for (const [namespace, names] of Object.entries(methods)) {
    engine[namespace] = Object.fromEntries(names.map(name => [name, (...argumentsList) => call(`${namespace}.${name}`, argumentsList)]));
  }
  const storeGet = engine.store.get;
  engine.store.get = (...argumentsList) => storeGet(...argumentsList).then(value => value === null ? undefined : value);
  engine.plugin = Object.freeze({ name: plugin.name, root: plugin.root });
  engine.ui = uiFor(plugin.id, call);
  engine.clock = clockFor(plugin.id, call);
  return Object.freeze(engine);
}
