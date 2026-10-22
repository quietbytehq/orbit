import type { Endpoint } from "$lib/api/types";

/** Human-readable place for an endpoint, for example "work:/Photos" or "/home/ana/Pictures". */
export function endpointLabel(endpoint: Endpoint): string {
  if (endpoint.kind === "local") return endpoint.path;
  return endpoint.path ? `${endpoint.remote}:/${endpoint.path}` : `${endpoint.remote}:`;
}
