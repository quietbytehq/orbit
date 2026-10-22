/** Remote paths use "/" and never start or end with one. "" is the remote root. */
export function normalizePath(path: string): string {
  return path
    .split("/")
    .filter((part) => part !== "" && part !== ".")
    .join("/");
}

/** Splits "a/b/c" into ["a/b", "c"]. A top-level name gives ["", name]. */
export function split(path: string): [string, string] {
  const normalized = normalizePath(path);
  const index = normalized.lastIndexOf("/");
  return index === -1 ? ["", normalized] : [normalized.slice(0, index), normalized.slice(index + 1)];
}

export function join(parent: string, name: string): string {
  const base = normalizePath(parent);
  const leaf = normalizePath(name);
  if (base === "") return leaf;
  if (leaf === "") return base;
  return `${base}/${leaf}`;
}

/** True when `path` is `folder` or lies inside it. */
export function isInside(path: string, folder: string): boolean {
  const target = normalizePath(path);
  const parent = normalizePath(folder);
  return target === parent || target.startsWith(`${parent}/`);
}
