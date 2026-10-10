import { h, Fragment } from "codex:mods/ui";
export { Fragment };
export const jsx = (component, properties, key) => {
  const { children, ...props } = properties ?? {};
  return h(component, { ...props, key }, children);
};
export const jsxs = jsx;
