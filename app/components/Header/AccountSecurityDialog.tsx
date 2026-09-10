"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import UserSignaturePad from "@/app/components/account/UserSignaturePad";
import PasswordInput from "@/app/components/auth/PasswordInput";
import PasswordRequirementsChecklist from "@/app/components/auth/PasswordRequirementsChecklist";
import SessionsManager from "@/app/components/account/SessionsManager";
import { useAppUser } from "@/app/hooks/useAppUser";
import { resolvePasskeyRegisterHints } from "@/app/lib/passkey-client-hints";
import { validatePasswordPolicy } from "@/app/lib/password-policy";

type Mode = "menu" | "password" | "email" | "sessions" | "signature" | "passkeys";

type Props = {
  open: boolean;
  onClose: () => void;
};

export default function AccountSecurityDialog({ open, onClose }: Props) {
  const router = useRouter();
  const { user, refresh } = useAppUser();
  const [mode, setMode] = useState<Mode>("menu");
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [newEmail, setNewEmail] = useState(user?.email ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [passkeys, setPasskeys] = useState<
    { id: string; name: string; createdAt: string | null }[]
  >([]);
  const [passkeysLoading, setPasskeysLoading] = useState(false);

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
    const policy = validatePasswordPolicy(newPassword);
    if (!policy.ok) {
      setError(policy.error);
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
      setSuccess("Mot de passe mis à jour. Redirection vers le tableau de bord…");
      setCurrentPassword("");
      setNewPassword("");
      setConfirmPassword("");
      window.setTimeout(() => {
        onClose();
        router.push("/dashboard");
        router.refresh();
      }, 1200);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
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

  async function loadPasskeys() {
    setPasskeysLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/account/passkeys", { credentials: "include" });
      const j = (await res.json()) as {
        error?: string;
        passkeys?: { id: string; name: string; createdAt: string | null }[];
      };
      if (!res.ok) throw new Error(j.error || "Impossible de charger les passkeys.");
      setPasskeys(j.passkeys ?? []);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setPasskeysLoading(false);
    }
  }

  async function addPasskeyFromDialog() {
    setBusy(true);
    setError(null);
    setSuccess(null);
    try {
      const hints = resolvePasskeyRegisterHints();
      const { authClient } = await import("@/app/lib/auth-client");
      const { data, error: regError } = await authClient.passkey.addPasskey({
        name: hints.name,
        authenticatorAttachment: hints.authenticatorAttachment,
      });
      if (regError) throw new Error(regError.message || "Enregistrement annulé.");
      if (!data) throw new Error("Aucune passkey enregistrée.");
      await fetch("/api/account/security-event", {
        method: "POST",
        credentials: "include",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ action: "passkey_registered" }),
      });
      setSuccess(
        hints.authenticatorAttachment === "platform"
          ? "Passkey enregistrée sur cet appareil."
          : "Passkey téléphone enregistrée.",
      );
      await loadPasskeys();
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function removePasskey(id: string) {
    if (!window.confirm("Supprimer cette passkey ?")) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/account/passkeys", {
        method: "DELETE",
        credentials: "include",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ id }),
      });
      const j = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(j.error || "Suppression impossible.");
      await loadPasskeys();
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
      onClick={success && mode === "password" ? undefined : closeAll}
      role="presentation"
    >
      <div
        className={`w-full overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-2xl ${
          mode === "sessions" || mode === "signature" || mode === "passkeys"
            ? "max-w-lg"
            : "max-w-md"
        }`}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="account-security-title"
      >
        <div className="flex items-center justify-between border-b border-slate-100 px-5 py-4">
          <h2 id="account-security-title" className="text-base font-bold text-slate-900">
            {mode === "menu" && "Sécurité"}
            {mode === "password" && "Changer le mot de passe"}
            {mode === "email" && "Changer l'e-mail de connexion"}
            {mode === "sessions" && "Appareils & sessions"}
            {mode === "signature" && "Ma signature"}
            {mode === "passkeys" && "Passkeys"}
          </h2>
          {success && mode === "password" ? null : (
            <button
              type="button"
              onClick={closeAll}
              className="rounded-lg px-2 py-1 text-sm font-semibold text-slate-500 hover:bg-slate-50"
            >
              Fermer
            </button>
          )}
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
                Changer mon e-mail de connexion
              </button>
              {user?.mfaSatisfied ? (
                <p className="rounded-xl border border-emerald-200 bg-emerald-50/70 px-4 py-3 text-sm text-emerald-900">
                  Sécurité renforcée active
                  {user.hasPasskey ? " (passkey)" : ""}
                  {user.twoFactorEnabled ? " (OTP)" : ""}.
                </p>
              ) : (
                <button
                  type="button"
                  onClick={() => {
                    closeAll();
                    window.location.href = "/auth/setup-2fa";
                  }}
                  className="flex w-full items-center gap-3 rounded-xl border border-slate-200 px-4 py-3 text-left text-sm font-semibold text-slate-800 transition hover:border-emerald-300 hover:bg-emerald-50/50"
                >
                  Configurer passkey / double authentification
                </button>
              )}
              <button
                type="button"
                onClick={() => {
                  resetForm();
                  setMode("passkeys");
                  void loadPasskeys();
                }}
                className="flex w-full items-center gap-3 rounded-xl border border-slate-200 px-4 py-3 text-left text-sm font-semibold text-slate-800 transition hover:border-emerald-300 hover:bg-emerald-50/50"
              >
                Gérer mes passkeys (téléphone)
              </button>
              <button
                type="button"
                onClick={() => {
                  resetForm();
                  setMode("signature");
                }}
                className="flex w-full items-center gap-3 rounded-xl border border-slate-200 px-4 py-3 text-left text-sm font-semibold text-slate-800 transition hover:border-emerald-300 hover:bg-emerald-50/50"
              >
                Ma signature (conventions & certificats)
              </button>
              <button
                type="button"
                onClick={() => {
                  resetForm();
                  setMode("sessions");
                }}
                className="flex w-full items-center gap-3 rounded-xl border border-slate-200 px-4 py-3 text-left text-sm font-semibold text-slate-800 transition hover:border-emerald-300 hover:bg-emerald-50/50"
              >
                Appareils & sessions connectées
              </button>
            </>
          )}

          {mode === "passkeys" && (
            <div className="space-y-3">
              <button
                type="button"
                onClick={() => {
                  resetForm();
                  setMode("menu");
                }}
                className="text-xs font-semibold text-slate-500 hover:text-slate-800"
              >
                ← Retour
              </button>
              <p className="text-sm text-slate-600">
                Sur PC : priorité téléphone (QR), pas Windows Hello. Sur mobile : Face ID /
                empreinte sur cet appareil.
              </p>
              {passkeysLoading ? (
                <p className="text-sm text-slate-500">Chargement…</p>
              ) : passkeys.length === 0 ? (
                <p className="text-sm text-slate-500">Aucune passkey enregistrée.</p>
              ) : (
                <ul className="space-y-2">
                  {passkeys.map((p) => (
                    <li
                      key={p.id}
                      className="flex items-center justify-between gap-2 rounded-xl border border-slate-200 px-3 py-2 text-sm"
                    >
                      <span className="font-medium text-slate-800">{p.name}</span>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => void removePasskey(p.id)}
                        className="text-xs font-semibold text-red-700 hover:underline disabled:opacity-50"
                      >
                        Supprimer
                      </button>
                    </li>
                  ))}
                </ul>
              )}
              <button
                type="button"
                disabled={busy}
                onClick={() => void addPasskeyFromDialog()}
                className="w-full rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2.5 text-sm font-bold text-white disabled:opacity-60"
              >
                {busy ? "En attente du téléphone…" : "Ajouter une passkey (téléphone)"}
              </button>
            </div>
          )}

          {mode === "sessions" && (
            <div className="space-y-3">
              <button
                type="button"
                onClick={() => {
                  resetForm();
                  setMode("menu");
                }}
                className="text-xs font-semibold text-slate-500 hover:text-slate-800"
              >
                ← Retour
              </button>
              <SessionsManager embedded />
            </div>
          )}

          {mode === "signature" && (
            <div className="space-y-3">
              <button
                type="button"
                onClick={() => {
                  resetForm();
                  setMode("menu");
                }}
                className="text-xs font-semibold text-slate-500 hover:text-slate-800"
              >
                ← Retour
              </button>
              <UserSignaturePad compact />
            </div>
          )}

          {mode === "password" && (
            <form onSubmit={submitPassword} className="space-y-3">
              {success ? (
                <p
                  className="rounded-xl border border-emerald-200 bg-emerald-50 px-3 py-3 text-sm font-medium text-emerald-800"
                  role="status"
                >
                  {success}
                </p>
              ) : (
                <>
                  <PasswordRequirementsChecklist password={newPassword} tone="slate" />
                  <label className="block space-y-1 text-sm">
                    <span className="font-medium text-slate-800">Mot de passe actuel</span>
                    <PasswordInput
                      autoComplete="current-password"
                      required
                      value={currentPassword}
                      onChange={(e) => setCurrentPassword(e.target.value)}
                      className="rounded-xl border border-slate-200 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
                    />
                  </label>
                  <label className="block space-y-1 text-sm">
                    <span className="font-medium text-slate-800">Nouveau mot de passe</span>
                    <PasswordInput
                      autoComplete="new-password"
                      required
                      minLength={12}
                      value={newPassword}
                      onChange={(e) => setNewPassword(e.target.value)}
                      className="rounded-xl border border-slate-200 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
                    />
                  </label>
                  <label className="block space-y-1 text-sm">
                    <span className="font-medium text-slate-800">Confirmer</span>
                    <PasswordInput
                      autoComplete="new-password"
                      required
                      minLength={12}
                      value={confirmPassword}
                      onChange={(e) => setConfirmPassword(e.target.value)}
                      className="rounded-xl border border-slate-200 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
                    />
                  </label>
                  {error ? <p className="text-sm text-red-600">{error}</p> : null}
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
                </>
              )}
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
                <PasswordInput
                  autoComplete="current-password"
                  required
                  value={currentPassword}
                  onChange={(e) => setCurrentPassword(e.target.value)}
                  className="rounded-xl border border-slate-200 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
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
