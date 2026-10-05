import "server-only";

import { getScolaRuntimeEnv } from "@/app/lib/scola-env";

/** Comptes seed — jamais en production réelle. */
export const DEMO_PARENT = {
  email: "parent@localhost.dev",
  password: "DevParentPass1!",
  label: "Parent (Leo JUSTIF · 4B)",
  redirect: "/quotidien?dev_tenant=default",
} as const;

export const DEMO_STAFF = {
  email: "admin@localhost.dev",
  password: "DevLocalPass1!",
  totpSecret: "DEVLOCALTOTPSECRET00000000000001",
  label: "Staff admin (intranet)",
  redirect: "/dashboard?dev_tenant=default",
} as const;

export type DemoRole = "parent" | "staff";

/**
 * Démo one-click autorisée hors prod pure.
 * `lab` (Scaleway hors prod) et `local` (Cloud Agent / npm run dev).
 */
export function isDemoClickAllowed(): boolean {
  return getScolaRuntimeEnv() !== "prod";
}

export function demoCredentials(role: DemoRole) {
  return role === "staff" ? DEMO_STAFF : DEMO_PARENT;
}
