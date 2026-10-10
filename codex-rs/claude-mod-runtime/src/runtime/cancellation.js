const controllers = new Map();

class ModAbortSignal {
  aborted = false;
  reason;
  listeners = new Set();
  addEventListener(name, callback) { if (name === "abort") this.listeners.add(callback); }
  removeEventListener(name, callback) { if (name === "abort") this.listeners.delete(callback); }
  throwIfAborted() { if (this.aborted) throw this.reason; }
  abort(reason) {
    if (this.aborted) return;
    this.aborted = true;
    this.reason = reason;
    for (const listener of this.listeners) listener({ type: "abort", target: this });
    this.listeners.clear();
  }
}

export function signalFor(id) {
  if (!controllers.has(id)) controllers.set(id, new ModAbortSignal());
  return controllers.get(id);
}

export function abort(id) {
  controllers.get(id)?.abort(Error("mod event cancelled"));
  controllers.delete(id);
}

export function finish(id) { controllers.delete(id); }
