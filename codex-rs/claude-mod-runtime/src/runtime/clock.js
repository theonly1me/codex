const timers = new Map();
let sequence = 0;

export function clockFor(plugin, call) {
  function schedule(delay, callback, repeat) {
    if (
      typeof callback !== "function" ||
      !Number.isFinite(delay) ||
      (repeat ? delay < 10 : delay < 0)
    ) {
      throw Error(
        "mod timers require a callback and a nonnegative delay; repeating timers require 10 ms",
      );
    }
    if (timers.size >= 4096) throw Error("mod timer limit reached");
    const id = ++sequence;
    timers.set(id, {
      plugin,
      callback,
      delay,
      repeat,
      due: Date.now() + delay,
      running: false,
    });
    return { cancel: () => timers.delete(id) };
  }
  return {
    now: () => call("clock.now", []),
    after: (delay, callback) => schedule(delay, callback, false),
    every: (delay, callback) => schedule(delay, callback, true),
    sleep: (delay) => new Promise((resolve) => schedule(delay, resolve, false)),
  };
}

export function tick(now) {
  for (const [id, timer] of timers) {
    if (timer.running || now < timer.due) continue;
    if (timer.repeat) timer.due = now + timer.delay;
    else timers.delete(id);
    timer.running = true;
    Promise.resolve()
      .then(timer.callback)
      .catch((error) => {
        timers.delete(id);
        return __modsHost({
          plugin: timer.plugin,
          method: "runtime.error",
          arguments: [String(error)],
        });
      })
      .finally(() => {
        timer.running = false;
      });
  }
}

export function stopTimers() {
  timers.clear();
}
