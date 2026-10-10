const callbacks = new Map();
const generations = new Map();
export const Fragment = "Fragment";

export function h(component, properties, ...children) {
  const props = properties ?? {};
  const flattened = children.flat(Infinity).filter(child => child !== null && child !== undefined && typeof child !== "boolean");
  if (typeof component === "function") return component({ ...props, children: flattened });
  const element = typeof component === "object" ? component.element : component;
  const plugin = typeof component === "object" ? component.plugin : undefined;
  return { element, plugin, props, children: flattened };
}

export function uiFor(plugin, call) {
  const names = ["Box", "Text", "Button", "Input", "Select", "Link", "Markdown", "Code", "Raster", "Image", "Client", "Svg"];
  const elements = Object.fromEntries(names.map(element => [element, properties => { const { children = [], ...props } = properties ?? {}; return { element, plugin, props, children }; }]));
  const request = (method, argumentsList) => call(`ui.${method}`, argumentsList);
  const notify = (method, argumentsList) => request(method, argumentsList).catch(error => call("runtime.error", [String(error)]));
  return {
    resolve: () => elements,
    invalidate: (...argumentsList) => notify("invalidate", argumentsList),
    status: (...argumentsList) => notify("status", argumentsList),
    toast: (...argumentsList) => notify("toast", argumentsList),
    log: (...argumentsList) => notify("log", argumentsList),
    notice: (...argumentsList) => notify("notice", argumentsList),
    open: (...argumentsList) => request("open", argumentsList),
    close: (...argumentsList) => request("close", argumentsList),
    panes: (...argumentsList) => request("panes", argumentsList),
    focus: (...argumentsList) => request("focus", argumentsList),
    scroll: (...argumentsList) => request("scroll", argumentsList),
    copy: (...argumentsList) => request("copy", argumentsList),
    ask: (...argumentsList) => request("ask", argumentsList),
    blit: (...argumentsList) => request("blit", argumentsList),
  };
}

export function encodeTree(value, renderId = "event", path = "root", owner) {
  if (typeof value === "function") {
    const id = `${owner ?? "mod"}:${renderId}:${generations.get(renderId) ?? 0}:${path}`;
    callbacks.set(id, value);
    return { callbackId: id };
  }
  if (Array.isArray(value)) return value.map((child, index) => encodeTree(child, renderId, `${path}.${index}`, owner));
  if (value && typeof value === "object") {
    const plugin = value.plugin ?? owner;
    return Object.fromEntries(Object.entries(value).map(([key, child]) => [key, encodeTree(child, renderId, `${path}.${key}`, plugin)]));
  }
  return value;
}

export async function invokeControl(id, value) {
  const callback = callbacks.get(id);
  if (!callback) throw Error(`mod control ${id} is no longer active`);
  return callback(value);
}

export function clearControls(renderId) {
  generations.set(renderId, (generations.get(renderId) ?? 0) + 1);
  for (const id of callbacks.keys()) if (id.includes(`:${renderId}:`)) callbacks.delete(id);
}
