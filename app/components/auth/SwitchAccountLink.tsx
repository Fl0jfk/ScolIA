"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { authClient } from "@/app/lib/auth-client";

/** Déconnexion puis page de connexion (évite la boucle MDP obligatoire / 2FA). */
export default function SwitchAccountLink({ className }: { className?: string }) {
  const router = useRouter();
  const [busy, setBusy] = useState(false);

  async function onClick() {
    setBusy(true);
    try {
      await authClient.signOut();
    } catch {
      /* on bascule quand même */
    }
    router.replace("/auth/sign-in");
    router.refresh();
  }

  return (
    <button
      type="button"
      onClick={() => void onClick()}
      disabled={busy}
      className={
        className ??
        "w-full text-center text-sm font-medium text-slate-600 underline-offset-2 hover:underline disabled:opacity-60"
      }
    >
      {busy ? "Déconnexion…" : "Utiliser un autre compte"}
    </button>
  );
}
