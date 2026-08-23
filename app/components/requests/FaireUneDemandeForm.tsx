"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import Link from "next/link";
import { useSessionUser } from "@/app/hooks/useAppUser";

const MAX_FILES = 12;

function deriveSubject(description: string): string {
  const line = description.trim().split(/\n/)[0]?.trim() || description.trim();
  if (line.length <= 80) return line;
  return `${line.slice(0, 77)}…`;
}

export type FaireUneDemandeSuccess = {
  kind: "sent" | "email";
  message: string;
  id?: string;
};

type Props = {
  variant?: "page" | "modal" | "inline";
  onSuccess?: (result: FaireUneDemandeSuccess) => void;
  mesDemandesHref?: string;
  /** Préfixe sujet (ex. [Demande RH]). */
  subjectPrefix?: string;
  /** Sujet fixe (ex. demande depuis l’annuaire) — ignore la dérivation depuis la description. */
  fixedSubject?: string;
  /** Force le routage ticketing (whitelist serveur). */
  forceRouteId?: string;
  heading?: string;
  intro?: string;
  placeholder?: string;
  hideIdentityCard?: boolean;
  /** Description préremplie (ex. demande depuis l’annuaire). */
  initialDescription?: string;
};

export default function FaireUneDemandeForm({
  variant = "page",
  onSuccess,
  mesDemandesHref = "/requests#mes-demandes",
  subjectPrefix,
  fixedSubject,
  forceRouteId,
  heading,
  intro,
  placeholder = "Expliquez ce dont vous avez besoin : réparation, document, information…",
  hideIdentityCard = false,
  initialDescription = "",
}: Props) {
  const { isLoaded, isSignedIn, user } = useSessionUser();
  const fileRef = useRef<HTMLInputElement>(null);
  const [description, setDescription] = useState(initialDescription);
  const [firstName, setFirstName] = useState("");
  const [lastName, setLastName] = useState("");
  const [email, setEmail] = useState("");
  const [phone, setPhone] = useState("");
  const [honeypot, setHoneypot] = useState("");
  const [files, setFiles] = useState<File[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<FaireUneDemandeSuccess | null>(null);

  useEffect(() => {
    if (initialDescription) setDescription(initialDescription);
  }, [initialDescription]);

  useEffect(() => {
    if (!isSignedIn || !user) return;
    setFirstName(user.firstName || "");
    setLastName(user.lastName || "");
    setEmail(user.primaryEmailAddress?.emailAddress || "");
    const meta = user.publicMetadata as Record<string, unknown>;
    const metaPhone =
      (typeof meta.phone === "string" ? meta.phone : "") || "";
    if (metaPhone) setPhone(metaPhone);
  }, [isSignedIn, user]);

  const canSubmit = useMemo(() => {
    if (description.trim().length < 15) return false;
    if (isSignedIn) return Boolean(firstName.trim() && lastName.trim() && email.trim());
    return (
      firstName.trim().length > 0 &&
      lastName.trim().length > 0 &&
      /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.trim()) &&
      phone.trim().length >= 8
    );
  }, [description, email, firstName, isSignedIn, lastName, phone]);

  const onPickFiles = (list: FileList | null) => {
    if (!list) return;
    setFiles((prev) => [...prev, ...Array.from(list)].slice(0, MAX_FILES));
  };

  const submit = async () => {
    if (!canSubmit || submitting) return;
    setSubmitting(true);
    setError(null);
    setSuccess(null);
    try {
      const rawSubject = fixedSubject?.trim() || deriveSubject(description);
      const subject = (
        fixedSubject?.trim()
          ? rawSubject
          : subjectPrefix
            ? `${subjectPrefix} ${rawSubject}`.trim()
            : rawSubject
      ).slice(0, 120);
      const fd = new FormData();
      fd.append("firstName", firstName.trim());
      fd.append("lastName", lastName.trim());
      fd.append("email", email.trim());
      fd.append("phone", isSignedIn && !phone.trim() ? "Non renseigné" : phone.trim());
      fd.append("subject", subject);
      fd.append("description", description.trim());
      if (forceRouteId) fd.append("forceRouteId", forceRouteId);
      fd.append("website", honeypot);
      for (const f of files) fd.append("files", f);
      const res = await fetch("/api/requests/create", { method: "POST", body: fd });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Envoi impossible.");
      if (data.needsEmailVerification) {
        const result: FaireUneDemandeSuccess = {
          kind: "email",
          message: data.message || "Consultez votre boîte mail pour confirmer l'envoi de votre demande.",
        };
        setSuccess(result);
        onSuccess?.(result);
        setDescription("");
        setFiles([]);
        return;
      }
      const result: FaireUneDemandeSuccess = {
        kind: "sent",
        message: "Votre demande a bien été enregistrée. Vous recevrez un e-mail de confirmation.",
        id: data.id,
      };
      setSuccess(result);
      onSuccess?.(result);
      setDescription("");
      setFiles([]);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur inattendue.");
    } finally {
      setSubmitting(false);
    }
  };

  const shellClass =
    variant === "modal"
      ? "space-y-4"
      : variant === "inline"
        ? "rounded-2xl border border-slate-200 bg-white p-5 md:p-6 shadow-sm space-y-4"
        : "rounded-3xl border border-slate-200 bg-white p-6 md:p-8 shadow-sm space-y-5";

  if (success) {
    return (
      <div className={`${variant === "modal" ? "" : shellClass} rounded-2xl border border-emerald-200 bg-emerald-50 p-5 space-y-3`}>
        <p className="font-bold text-emerald-900">{success.kind === "email" ? "Vérifiez votre e-mail" : "Demande envoyée"}</p>
        <p className="text-sm text-emerald-800 leading-relaxed">{success.message}</p>
        {success.id ? <p className="text-xs text-emerald-700 font-mono">Réf. {success.id}</p> : null}
        <div className="flex flex-wrap gap-2 pt-1">
          <button
            type="button"
            onClick={() => setSuccess(null)}
            className="px-4 py-2 rounded-full bg-white border border-emerald-200 text-sm font-bold text-emerald-800 hover:bg-emerald-100 transition"
          >
            Nouvelle demande
          </button>
          {isSignedIn ? (
            <Link
              href={mesDemandesHref}
              className="px-4 py-2 rounded-full bg-emerald-600 text-white text-sm font-bold hover:bg-emerald-700 transition"
            >
              Voir mes demandes
            </Link>
          ) : null}
        </div>
      </div>
    );
  }

  return (
    <form
      className={`relative ${shellClass}`}
      onSubmit={(e) => {
        e.preventDefault();
        void submit();
      }}
    >
      {!isLoaded ? (
        <p className="text-sm text-slate-500 animate-pulse">Chargement…</p>
      ) : (
        <>
          <div
            className="pointer-events-none absolute left-0 top-0 -z-10 h-0 w-0 overflow-hidden opacity-0"
            aria-hidden
          >
            <label>
              Site web
              <input
                tabIndex={-1}
                autoComplete="off"
                value={honeypot}
                onChange={(e) => setHoneypot(e.target.value)}
              />
            </label>
          </div>
          {heading || intro ? (
            <div className="space-y-1">
              {heading ? <h3 className="text-lg font-black text-slate-900">{heading}</h3> : null}
              {intro ? <p className="text-sm text-slate-600 leading-relaxed">{intro}</p> : null}
            </div>
          ) : null}

          {!isSignedIn ? (
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <div>
                <label className="text-[10px] font-black uppercase text-slate-400">Prénom</label>
                <input
                  value={firstName}
                  onChange={(e) => setFirstName(e.target.value)}
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm font-medium"
                  required
                />
              </div>
              <div>
                <label className="text-[10px] font-black uppercase text-slate-400">Nom</label>
                <input
                  value={lastName}
                  onChange={(e) => setLastName(e.target.value)}
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm font-medium"
                  required
                />
              </div>
              <div>
                <label className="text-[10px] font-black uppercase text-slate-400">E-mail</label>
                <input
                  type="email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm font-medium"
                  required
                />
              </div>
              <div>
                <label className="text-[10px] font-black uppercase text-slate-400">Téléphone</label>
                <input
                  value={phone}
                  onChange={(e) => setPhone(e.target.value)}
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm font-medium"
                  required
                />
              </div>
            </div>
          ) : hideIdentityCard ? null : (
            <div className="rounded-2xl bg-slate-50 border border-slate-100 px-4 py-3 text-sm">
              <p className="font-bold text-slate-800">{user?.fullName || `${firstName} ${lastName}`.trim()}</p>
              <p className="text-slate-500 mt-0.5">{email}</p>
            </div>
          )}

          <div>
            <label className="text-[10px] font-black uppercase text-slate-400">Votre demande</label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={variant === "modal" ? 5 : 6}
              placeholder={placeholder}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-3 text-sm font-medium resize-y min-h-[120px]"
              required
            />
            <p className="text-[11px] text-slate-400 mt-1">Minimum 15 caractères.</p>
          </div>

          <div>
            <label className="text-[10px] font-black uppercase text-slate-400">Photos ou documents (optionnel)</label>
            <input
              ref={fileRef}
              type="file"
              multiple
              accept="image/*,.pdf,.doc,.docx,.xls,.xlsx"
              className="hidden"
              onChange={(e) => onPickFiles(e.target.files)}
            />
            <button
              type="button"
              onClick={() => fileRef.current?.click()}
              className="mt-2 w-full py-2.5 rounded-xl border-2 border-dashed border-slate-200 text-sm font-bold text-slate-500 hover:border-blue-300 hover:text-blue-600 transition"
            >
              Ajouter des fichiers ({files.length}/{MAX_FILES})
            </button>
            {files.length > 0 ? (
              <ul className="mt-2 space-y-1">
                {files.map((f, i) => (
                  <li
                    key={`${f.name}-${i}`}
                    className="flex items-center justify-between text-xs text-slate-600 bg-slate-50 rounded-lg px-3 py-2"
                  >
                    <span className="truncate">{f.name}</span>
                    <button
                      type="button"
                      className="text-red-500 font-bold ml-2"
                      onClick={() => setFiles((prev) => prev.filter((_, j) => j !== i))}
                    >
                      Retirer
                    </button>
                  </li>
                ))}
              </ul>
            ) : null}
          </div>

          {error ? <p className="text-sm font-medium text-red-600">{error}</p> : null}

          {!isSignedIn ? (
            <p className="text-xs text-slate-500 leading-relaxed">
              Après envoi, vous recevrez un e-mail avec un lien de confirmation. La demande ne partira qu&apos;après ce clic.
            </p>
          ) : null}

          <button
            type="submit"
            disabled={!canSubmit || submitting}
            className="w-full py-3.5 rounded-2xl bg-blue-600 text-white font-black text-sm hover:bg-blue-700 disabled:opacity-40 disabled:cursor-not-allowed transition shadow-lg shadow-blue-200/50"
          >
            {submitting ? "Envoi en cours…" : isSignedIn ? "Envoyer ma demande" : "Recevoir le lien de confirmation"}
          </button>
        </>
      )}
    </form>
  );
}
