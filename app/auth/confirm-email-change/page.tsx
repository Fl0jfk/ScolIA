"use client";

import { Suspense, useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import Link from "next/link";

function ConfirmEmailChangeInner() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const token = searchParams.get("token")?.trim() || "";
  const [status, setStatus] = useState<"loading" | "ok" | "error">("loading");
  const [message, setMessage] = useState("Confirmation en cours…");

  useEffect(() => {
    if (!token) {
      setStatus("error");
      setMessage("Lien invalide.");
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        const res = await fetch("/api/account/confirm-email", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ token }),
        });
        const j = (await res.json()) as { error?: string; email?: string };
        if (cancelled) return;
        if (!res.ok) throw new Error(j.error || "Confirmation impossible.");
        setStatus("ok");
        setMessage(`E-mail confirmé : ${j.email ?? ""}. Vous pouvez vous reconnecter.`);
        setTimeout(() => router.replace("/auth/sign-in"), 2500);
      } catch (err) {
        if (cancelled) return;
        setStatus("error");
        setMessage(err instanceof Error ? err.message : "Erreur");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [token, router]);

  return (
    <div className="flex min-h-[calc(100vh-3.5rem)] items-center justify-center px-4 py-10">
      <div className="w-full max-w-md rounded-2xl border border-slate-200 bg-white p-8 text-center shadow-xl">
        <h1 className="text-xl font-semibold text-slate-900">Confirmation d’e-mail</h1>
        <p
          className={`mt-4 text-sm ${status === "error" ? "text-red-600" : "text-slate-600"}`}
          role="status"
        >
          {message}
        </p>
        {status !== "loading" ? (
          <Link
            href="/auth/sign-in"
            className="mt-6 inline-flex rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2 text-sm font-bold text-white"
          >
            Connexion
          </Link>
        ) : null}
      </div>
    </div>
  );
}

export default function ConfirmEmailChangePage() {
  return (
    <Suspense fallback={<p className="p-10 text-center text-sm">Chargement…</p>}>
      <ConfirmEmailChangeInner />
    </Suspense>
  );
}
