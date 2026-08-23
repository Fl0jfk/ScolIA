import { normalizeHostname } from "@/app/lib/tenant-registry";

const LOCAL_DEV_HOSTS = new Set(["localhost", "127.0.0.1"]);

export function isLocalDevHostname(hostname: string): boolean {
  return LOCAL_DEV_HOSTS.has(normalizeHostname(hostname));
}
