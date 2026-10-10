import { register, dispatch, stream, clearHandlers } from "codex:mods/events";
import {
  h,
  Fragment,
  encodeTree,
  invokeControl,
  clearControls,
} from "codex:mods/ui";
import { tick, stopTimers } from "codex:mods/clock";
import * as state from "codex:mods/state";
import { abort, finish } from "codex:mods/cancellation";
import { btoa, atob } from "codex:mods/encoding";

globalThis.structuredClone = (value) => JSON.parse(JSON.stringify(value));
globalThis.btoa = btoa;
globalThis.atob = atob;
globalThis.__mods = {
  register,
  dispatch,
  stream,
  h,
  Fragment,
  tick,
  state,
  abort,
  async begin(id, name, input) {
    try {
      let result;
      if (name === "runtime.control") {
        result = await invokeControl(input.callbackId, input.value);
      } else if (name === "turn.step") {
        const iterator = stream(name, input, id);
        for await (const chunk of iterator) {
          await __modsHost({
            dispatchId: id,
            method: "__next",
            arguments: [{ streamOperation: "emit", chunk }],
          });
        }
        result = await iterator.result;
      } else {
        if (name === "ui.close") clearControls(input.id);
        if (name === "ui.render")
          clearControls(input.requestId ?? input.component);
        result = await dispatch(name, input, id);
      }
      __modsSettle(
        id,
        encodeTree(result, input.requestId ?? input.component),
        false,
      );
    } catch (error) {
      __modsSettle(id, String(error?.stack ?? error), true);
    } finally {
      finish(id);
    }
  },
  shutdown() {
    stopTimers();
    clearHandlers();
  },
};
