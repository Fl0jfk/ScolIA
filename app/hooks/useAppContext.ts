"use client";

import { useContext } from "react";
import { AdminBootstrapContext } from "@/app/contexts/admin-bootstrap";

export type { AppContextPayload } from "@/app/contexts/admin-bootstrap";

export function useAppContext() {
  const bootstrap = useContext(AdminBootstrapContext);
  if (!bootstrap) { throw new Error("useAppContext doit être utilisé dans AdminBootstrapProvider")}
  return {
    data: bootstrap.appContext,
    loading: bootstrap.loading,
    error: bootstrap.error,
  };
}
