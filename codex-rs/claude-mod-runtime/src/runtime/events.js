import { engineFor } from "codex:mods/api";
import { signalFor } from "codex:mods/cancellation";

const handlers = [];
const events = new Set([
  "session.start", "session.end", "session.measure", "classic.SessionStart", "turn.start", "turn.complete", "turn.step",
  "prompt.submit", "prompt.compose", "tool.call", "command.run",
  "config.set", "ui.render", "ui.close", "ui.focus", "*",
]);

function matches(filter, event) {
  return Object.entries(filter ?? {}).every(([key, wanted]) => {
    const actual = event[key];
    if (Array.isArray(wanted)) return wanted.includes(actual);
    return wanted === actual;
  });
}

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

export function register(plugin, callback, options) {
  const on = (name, filter, hook) => {
    if (!events.has(name)) throw Error(`unsupported mod event ${name}`);
    if (typeof filter === "function") { hook = filter; filter = undefined; }
    if (typeof hook !== "function") throw Error(`mod event ${name} has no handler`);
    const record = { plugin, name, filter, hook, failure: undefined };
    handlers.push(record);
    return { catch: handler => { record.failure = handler; } };
  };
  callback(on, freeze(options));
}

function selected(name, event) {
  return handlers.filter(record => (record.name === name || record.name === "*")
    && (!event.targetPluginId || record.plugin.id === event.targetPluginId) && matches(record.filter, event));
}

function continuation(run, origin, dispatchId) {
  let called = false;
  const next = input => {
    if (called) throw Error("mod continuation called more than once");
    called = true;
    return run(input);
  };
  Object.defineProperty(next, "called", { get: () => called });
  next.origin = Object.freeze({ plugin: origin ?? "engine", tier: origin ? "user" : "core" });
  next.budget = Object.freeze({ ms: 60000, remainingMs: 60000 });
  next.signal = signalFor(dispatchId);
  return next;
}

export async function dispatch(name, event, dispatchId) {
  const records = selected(name, event);
  async function run(index, input) {
    if (index === records.length) {
      return __modsHost({ dispatchId, method: "__next", arguments: [input] });
    }
    const record = records[index];
    const engine = engineFor(record.plugin, dispatchId);
    const next = continuation(changed => run(index + 1, changed), records[index + 1]?.plugin.id, dispatchId);
    try {
      return await record.hook(engine, freeze(input), next);
    } catch (error) {
      if (!record.failure) throw error;
      next.error = Object.freeze({ kind: "throw", message: String(error) });
      return record.failure(engine, freeze(input), next);
    }
  }
  return run(0, event);
}

export function stream(name, event, dispatchId) {
  const records = selected(name, event);
  function run(index, input) {
    let resolveResult;
    let rejectResult;
    const result = new Promise((resolve, reject) => { resolveResult = resolve; rejectResult = reject; });
    result.catch(() => undefined);
    const iterator = (async function* () {
      try {
        if (index === records.length) {
          await __modsHost({ dispatchId, method: "__next", arguments: [{ streamOperation: "start", input }] });
          while (true) {
            const item = await __modsHost({ dispatchId, method: "__next", arguments: [{ streamOperation: "pull" }] });
            if (item.done) { resolveResult(item.result); return item.result; }
            yield item.value;
          }
        }
        const record = records[index];
        const engine = engineFor(record.plugin, dispatchId);
        const next = continuation(changed => run(index + 1, changed), records[index + 1]?.plugin.id, dispatchId);
        const generator = record.hook(engine, freeze(input), next);
        while (true) {
          const item = await generator.next();
          if (item.done) { resolveResult(item.value); return item.value; }
          yield item.value;
        }
      } catch (error) { rejectResult(error); throw error; }
    })();
    iterator.result = result;
    return iterator;
  }
  return run(0, event);
}

export function clearHandlers() {
  handlers.length = 0;
}
