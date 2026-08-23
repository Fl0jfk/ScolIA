"use client";

import { useState } from "react";
import { useAppUser } from "@/app/hooks/useAppUser";

type Mode = "menu" | "password" | "email";

type Props = {
  open: boolean;
  onClose: () => void;
};

export default function AccountSecurityDialog({ open, onClose }: Props) {
  const { user, refresh } = useAppUser();
  const [mode, setMode] = useState<Mode>("menu");
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [newEmail, setNewEmail] = useState(user?.email ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  if (!open) return null;

  function resetForm() {
    setCurrentPassword("");
    setNewPassword("");
    setConfirmPassword("");
    setNewEmail(user?.email ?? "");
    setError(null);
    setSuccess(null);
    setBusy(false);
  }

  function closeAll() {
    resetForm();
    setMode("menu");
    onClose();
  }

  async function submitPassword(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setSuccess(null);
    if (newPassword.length < 10) {
      setError("Le nouveau mot de passe doit contenir au moins 10 caractères.");
      return;
    }
    if (newPassword !== confirmPassword) {
      setError("La confirmation ne correspond pas.");
      return;
    }
    setBusy(true);
    try {
      const res = await fetch("/api/account/security", {
        method: "POST",
        credentials: "include",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          action: "password",
          currentPassword,
          newPassword,
        }),
      });
      const j = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(j.error || "Échec du changement de mot de passe.");
      setSuccess("Mot de passe mis à jour.");
      setCurrentPassword("");
      setNewPassword("");
      setConfirmPassword("");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function submitEmail(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setSuccess(null);
    setBusy(true);
    try {
      const res = await fetch("/api/account/security", {
        method: "POST",
        credentials: "include",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          action: "email",
          currentPassword,
          newEmail: newEmail.trim(),
        }),
      });
      const j = (await res.json()) as {
        error?: string;
        email?: string;
        message?: string;
        warning?: string;
        mode?: string;
      };
      if (!res.ok) throw new Error(j.error || "Échec du changement d’e-mail.");
      setSuccess(j.message || j.warning || `E-mail mis à jour : ${j.email ?? newEmail}`);
      setCurrentPassword("");
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div
      className="fixed inset-0 z-[80] flex items-center justify-center bg-slate-900/40 p-4 backdrop-blur-[2px]"
      onClick={closeAll}
      role="presentation"
    >
      <div
        className="w-full max-w-md overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-2xl"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="account-security-title"
      >
        <div className="flex items-center justify-between border-b border-slate-100 px-5 py-4">
          <h2 id="account-security-title" className="text-base font-bold text-slate-900">
            {mode === "menu" && "Mon compte"}
            {mode === "password" && "Changer le mot de passe"}
            {mode === "email" && "Changer l’e-mail de connexion"}
          </h2>
          <button
            type="button"
            onClick={closeAll}
            className="rounded-lg px-2 py-1 text-sm font-semibold text-slate-500 hover:bg-slate-50"
          >
            Fermer
          </button>
        </div>

        <div className="space-y-4 px-5 py-4">
          {mode === "menu" && (
            <>
              <p className="text-sm text-slate-600">
                Connecté en tant que{" "}
                <span className="font-semibold text-slate-900">{user?.email ?? "—"}</span>
              </p>
              <button
                type="button"
                onClick={() => {
                  resetForm();
                  setMode("password");
                }}
                className="flex w-full items-center gap-3 rounded-xl border border-slate-200 px-4 py-3 text-left text-sm font-semibold text-slate-800 transition hover:border-emerald-300 hover:bg-emerald-50/50"
              >
                <span aria-hidden>🔑</span>
                Changer mon mot de passe
              </button>
              <button
                type="button"
                onClick={() => {
                  resetForm();
                  setNewEmail(user?.email ?? "");
                  setMode("email");
                }}
                className="flex w-full items-center gap-3 rounded-xl border border-slate-200 px-4 py-3 text-left text-sm font-semibold text-slate-800 transition hover:border-emerald-300 hover:bg-emerald-50/50"
              >
                <span aria-hidden>✉️</span>
                Changer mon e-mail de connexion
              </button>
            </>
          )}

          {mode === "password" && (
            <form onSubmit={submitPassword} className="space-y-3">
              <label className="block space-y-1 text-sm">
                <span className="font-medium text-slate-800">Mot de passe actuel</span>
                <input
                  type="password"
                  autoComplete="current-password"
                  required
                  value={currentPassword}
                  onChange={(e) => setCurrentPassword(e.target.value)}
                  className="w-full rounded-xl border border-slate-200 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
                />
              </label>
              <label className="block space-y-1 text-sm">
                <span className="font-medium text-slate-800">Nouveau mot de passe</span>
                <input
                  type="password"
                  autoComplete="new-password"
                  required
                  minLength={10}
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  className="w-full rounded-xl border border-slate-200 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
                />
              </label>
              <label className="block space-y-1 text-sm">
                <span className="font-medium text-slate-800">Confirmer</span>
                <input
                  type="password"
                  autoComplete="new-password"
                  required
                  minLength={10}
                  value={confirmPassword}
                  onChange={(e) => setConfirmPassword(e.target.value)}
                  className="w-full rounded-xl border border-slate-200 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
                />
              </label>
              {error ? <p className="text-sm text-red-600">{error}</p> : null}
              {success ? <p className="text-sm text-emerald-700">{success}</p> : null}
              <div className="flex gap-2 pt-1">
                <button
                  type="button"
                  onClick={() => {
                    resetForm();
                    setMode("menu");
                  }}
                  className="rounded-xl border border-slate-200 px-4 py-2 text-sm font-semibold text-slate-700"
                >
                  Retour
                </button>
                <button
                  type="submit"
                  disabled={busy}
                  className="flex-1 rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2 text-sm font-bold text-white disabled:opacity-60"
                >
                  {busy ? "Enregistrement…" : "Enregistrer"}
                </button>
              </div>
            </form>
          )}

          {mode === "email" && (
            <form onSubmit={submitEmail} className="space-y-3">
              <label className="block space-y-1 text-sm">
                <span className="font-medium text-slate-800">Nouvel e-mail</span>
                <input
                  type="email"
                  autoComplete="email"
                  required
                  value={newEmail}
                  onChange={(e) => setNewEmail(e.target.value)}
                  className="w-full rounded-xl border border-slate-200 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
                />
              </label>
              <label className="block space-y-1 text-sm">
                <span className="font-medium text-slate-800">Mot de passe actuel (confirmation)</span>
                <input
                  type="password"
                  autoComplete="current-password"
                  required
                  value={currentPassword}
                  onChange={(e) => setCurrentPassword(e.target.value)}
                  className="w-full rounded-xl border border-slate-200 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
                />
              </label>
              {error ? <p className="text-sm text-red-600">{error}</p> : null}
              {success ? <p className="text-sm text-emerald-700">{success}</p> : null}
              {!success ? (
                <p className="text-xs text-slate-500">
                  Un e-mail de confirmation sera envoyé à la nouvelle adresse (lien valable
                  1&nbsp;h). Votre ancienne adresse sera aussi notifiée.
                </p>
              ) : null}
              <div className="flex gap-2 pt-1">
                <button
                  type="button"
                  onClick={() => {
                    resetForm();
                    setMode("menu");
                  }}
                  className="rounded-xl border border-slate-200 px-4 py-2 text-sm font-semibold text-slate-700"
                >
                  Retour
                </button>
                <button
                  type="submit"
                  disabled={busy}
                  className="flex-1 rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2 text-sm font-bold text-white disabled:opacity-60"
                >
                  {busy ? "Enregistrement…" : "Enregistrer"}
                </button>
              </div>
            </form>
          )}
        </div>
      </div>
    </div>
  );
}
