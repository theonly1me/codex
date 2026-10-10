export function atom(point, initial) {
  return Object.freeze({ point, initial });
}

export function derive(source, map) {
  return Object.freeze({ source, map });
}

export function memberOf(group, member) {
  return Object.freeze({ group, member });
}

export async function read(engine, reference) {
  if (reference.source)
    return reference.map(await read(engine, reference.source));
  if (reference.group)
    return (await read(engine, reference.group))[reference.member];
  const stored = await engine.state.get(reference.point);
  return stored === undefined || stored === null
    ? structuredClone(reference.initial)
    : stored;
}

const updates = new Map();

export async function update(engine, reference, change) {
  const key = `${reference.point.plugin}:${reference.point.key}`;
  const previous = updates.get(key) ?? Promise.resolve();
  const operation = previous.then(async () => {
    const value = change(await read(engine, reference));
    await engine.state.set(reference.point, value);
    return value;
  });
  const settled = operation.catch(() => undefined);
  updates.set(key, settled);
  try {
    return await operation;
  } finally {
    if (updates.get(key) === settled) updates.delete(key);
  }
}
