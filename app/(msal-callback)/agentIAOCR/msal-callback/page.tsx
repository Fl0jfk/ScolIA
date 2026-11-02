"use client";

import { useLayoutEffect, useState } from "react";
import { PublicClientApplication } from "@azure/msal-browser";
import {
  buildOneDriveMsalConfig,
  consumeMsalReturnPath,
  fetchMicrosoftOneDrivePublicConfig,
} from "@/app/lib/msal-onedrive-client";

function isEmbeddedMsalFrame(): boolean {
  if (typeof window === "undefined") return false;
  try {
    return window.parent !== window;
  } catch {
    return true;
  }
}

/**
 * Retour OAuth MSAL (redirect top-level / popup).
 * Ne doit RIEN faire dans l'iframe silent de acquireTokenSilent :
 * le parent MSAL lit le hash ; une navigation ici relançait MSAL en boucle
 * (erreurs Chrome « sandboxed … allow-top-navigation »).
 */
export default function OneDriveMsalCallbackPage() {
  const [message, setMessage] = useState("Connexion Microsoft en cours…");
  const [embedded, setEmbedded] = useState(false);

  useLayoutEffect(() => {
    if (isEmbeddedMsalFrame()) {
      setEmbedded(true);
      return;
    }

    let cancelled = false;
    (async () => {
      try {
        const ms = await fetchMicrosoftOneDrivePublicConfig();
        if (!ms) throw new Error("OneDrive n'est pas configuré pour cet établissement.");
        const app = new PublicClientApplication(buildOneDriveMsalConfig(ms));
        await app.initialize();
        await app.handleRedirectPromise({ navigateToLoginRequestUrl: false });
        if (cancelled) return;
        if (window.opener && !window.opener.closed) {
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
    return () => {
      cancelled = true;
    };
  }, []);

  if (embedded) return null;

  return (
    <main className="min-h-[40vh] flex items-center justify-center p-8">
      <p className="text-sm font-medium text-slate-600">{message}</p>
    </main>
  );
}
