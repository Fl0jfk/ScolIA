"use client";

import { useLayoutEffect, useState } from "react";
import { PublicClientApplication } from "@azure/msal-browser";
import { buildOneDriveMsalConfig, consumeMsalReturnPath, fetchMicrosoftOneDrivePublicConfig} from "@/app/lib/msal-onedrive-client";

export default function OneDriveMsalCallbackPage() {
  const [message, setMessage] = useState("Connexion Microsoft en cours…");
  useLayoutEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const ms = await fetchMicrosoftOneDrivePublicConfig();
        if (!ms) throw new Error("OneDrive n'est pas configuré pour cet établissement.");
        const app = new PublicClientApplication(buildOneDriveMsalConfig(ms));
        await app.initialize();
        await app.handleRedirectPromise();
        if (cancelled) return;
        if (typeof window !== "undefined" && window.opener && !window.opener.closed) {
          window.close();
          return;
        }
        window.location.replace(consumeMsalReturnPath());
      } catch (err: unknown) {
        if (cancelled) return;
        const detail = err instanceof Error ? err.message : "Échec de la connexion.";
        setMessage(detail);
        window.setTimeout(() => {
          window.location.replace(consumeMsalReturnPath());
        }, 2500);
      }
    })();
    return () => { cancelled = true}}, []);
  return (
    <main className="min-h-[40vh] flex items-center justify-center p-8">
      <p className="text-sm font-medium text-slate-600">{message}</p>
    </main>
  );
}
