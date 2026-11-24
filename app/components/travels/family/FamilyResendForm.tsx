"use client";

import { useState } from "react";

/** Renvoi du lien personnel : la réponse est la même que l'adresse soit connue ou non. */
export function FamilyResendForm() {
  const [email, setEmail] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/voyages/famille/resend", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email }),
      });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Demande impossible");
      setMessage(j.message);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Demande impossible");
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="mx-auto max-w-xl p-4 sm:p-8">
      <form onSubmit={submit} className="space-y-4 rounded-2xl border-2 border-slate-200 bg-white p-6">
        <h1 className="text-xl font-black text-slate-900">Recevoir mon lien</h1>
        <p className="text-sm text-slate-600">
          Saisissez l&apos;adresse e-mail que l&apos;établissement a pour vous. Si elle est connue, vous recevrez un nouveau lien pour répondre au voyage scolaire.
        </p>
        <input
          type="email"
          required
          autoComplete="email"
          value={email}
          onChange={(ev) => setEmail(ev.target.value)}
          placeholder="votre.adresse@exemple.fr"
          className="w-full rounded-lg border p-3 text-sm"
        />
        {error ? <p role="alert" className="text-sm font-semibold text-rose-700">{error}</p> : null}
        {message ? <p role="status" className="text-sm font-semibold text-emerald-700">{message}</p> : null}
        <button disabled={busy} className="rounded-xl bg-indigo-600 px-5 py-3 text-sm font-bold text-white disabled:opacity-50">
          {busy ? "Un instant…" : "Recevoir mon lien"}
        </button>
      </form>
    </main>
  );
}
