"use client";

/**
 * Compléments de la page des familles (lot 3) : désaccord du second responsable (« Je ne suis pas d'accord »),
 * dépôt des pièces « à fournir avant le départ » après la signature, messages neutres sur la situation de l'élève.
 */

import { useState } from "react";
import type { PublicFamilyView } from "@/app/lib/travels-v2-registration";
import { dateFr } from "@/app/lib/travels-family-page";

const base = (token: string) => `/api/voyages/famille/${encodeURIComponent(token)}`;

type Props = { token: string; view: PublicFamilyView; onView: (v: PublicFamilyView) => void };

/** Message neutre : jamais de détail sur le désaccord d'un autre responsable. */
export function StudentStateNotice({ view }: { view: PublicFamilyView }) {
  if (view.studentState === "suspens") {
    return (
      <p role="status" className="rounded-2xl border-2 border-amber-300 bg-amber-50 p-4 text-sm font-semibold text-amber-950">
        L&apos;inscription de {view.studentName} est en cours de vérification par la direction de l&apos;établissement. Vous serez informé(e) de la suite.
      </p>
    );
  }
  if (view.studentState === "retire") {
    return (
      <p role="status" className="rounded-2xl border-2 border-rose-300 bg-rose-50 p-4 text-sm font-semibold text-rose-950">
        L&apos;inscription de {view.studentName} a été annulée par la direction. Contactez l&apos;établissement pour toute question.
      </p>
    );
  }
  if (view.studentState === "desiste") {
    return (
      <p role="status" className="rounded-2xl border-2 border-zinc-300 bg-zinc-50 p-4 text-sm font-semibold text-zinc-900">
        Le désistement de {view.studentName} a été enregistré par l&apos;établissement.
      </p>
    );
  }
  return null;
}

/** Désaccord confirmé par un code reçu par e-mail (comme une signature). */
export function FamilyObjectionBox({ token, view, onView }: Props) {
  const [open, setOpen] = useState(false);
  const [code, setCode] = useState("");
  const [name, setName] = useState("");
  const [reason, setReason] = useState("");
  const [info, setInfo] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  if (view.objection) {
    return (
      <p role="status" className="rounded-2xl border-2 border-amber-300 bg-amber-50 p-4 text-sm text-amber-950">
        {view.objection.decision === "maintenir"
          ? "Votre désaccord a été examiné : la direction a maintenu l'inscription."
          : view.objection.decision === "retirer"
            ? "Votre désaccord a été examiné : la direction a retiré l'élève du voyage."
            : `Votre désaccord a été transmis à la direction, qui prendra contact. L'inscription de ${view.studentName} est en suspens.`}
      </p>
    );
  }
  if (!view.canObject) return null;

  const sendCode = async () => {
    setBusy(true);
    setError(null);
    setInfo(null);
    try {
      const res = await fetch(`${base(token)}/code`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ purpose: "objection" }),
      });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Envoi du code impossible");
      setInfo(`Un code à 6 chiffres vient d'être envoyé à ${j.maskedEmail}.`);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Envoi du code impossible");
    } finally {
      setBusy(false);
    }
  };

  const submit = async () => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`${base(token)}/objection`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ signerName: name, code, ...(reason.trim() ? { reason: reason.trim() } : {}) }),
      });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error + (typeof j.attemptsLeft === "number" ? ` (${j.attemptsLeft} essai(s) restant(s))` : ""));
      onView(j.view as PublicFamilyView);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Enregistrement impossible");
    } finally {
      setBusy(false);
    }
  };

  const inputCls = "rounded-lg border bg-white p-2 text-sm";
  return (
    <section className="space-y-3 rounded-2xl border-2 border-red-200 bg-red-50/60 p-4 text-sm">
      <p className="font-semibold text-red-950">
        Un responsable a inscrit {view.studentName} à ce voyage. Sans action de votre part, l&apos;inscription est maintenue.
      </p>
      {!open ? (
        <button type="button" onClick={() => setOpen(true)} className="rounded-xl bg-red-700 px-4 py-2 text-sm font-bold text-white">
          Je ne suis pas d&apos;accord
        </button>
      ) : (
        <div className="space-y-3">
          <p className="text-xs text-slate-700">
            Votre désaccord sera transmis à la direction, qui tranchera. En attendant, l&apos;élève est « en suspens ». Il est confirmé par un code envoyé à votre adresse e-mail.
          </p>
          {error ? <p role="alert" className="rounded-lg bg-rose-50 p-2 font-semibold text-rose-700">{error}</p> : null}
          <div className="flex flex-wrap items-center gap-2">
            <button type="button" onClick={() => void sendCode()} disabled={busy} className="rounded-lg bg-slate-200 px-3 py-2 text-xs font-bold text-slate-700 disabled:opacity-50">
              Recevoir mon code
            </button>
            <input
              inputMode="numeric"
              autoComplete="one-time-code"
              maxLength={6}
              value={code}
              onChange={(e) => setCode(e.target.value.replace(/\D/g, ""))}
              placeholder="Code à 6 chiffres"
              aria-label="Code à 6 chiffres reçu par e-mail"
              className={`w-40 ${inputCls}`}
            />
          </div>
          {info ? <p className="text-xs text-emerald-700">{info}</p> : null}
          <input value={name} onChange={(e) => setName(e.target.value)} maxLength={100} autoComplete="name" placeholder="Nom et prénom du responsable" className={`w-full ${inputCls}`} />
          <textarea
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            maxLength={500}
            rows={2}
            placeholder="Motif (facultatif, lu uniquement par la direction)"
            className={`w-full ${inputCls}`}
          />
          <button
            type="button"
            onClick={() => void submit()}
            disabled={busy || name.trim().length < 2 || !/^\d{6}$/.test(code)}
            className="rounded-xl bg-red-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-40"
          >
            {busy ? "Un instant…" : "Confirmer mon désaccord"}
          </button>
        </div>
      )}
    </section>
  );
}

/** Pièces « à fournir avant le départ » : dépôt possible après la signature, jusqu'à la date de la pièce. */
export function FamilyAfterSignDocs({ token, view, onView }: Props) {
  const [uploading, setUploading] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const docs = view.model.documents.filter((d) => view.uploadableAfterSign.includes(d.id));
  if (docs.length === 0) return null;

  const upload = async (docId: string, file: File | undefined) => {
    if (!file) return;
    setUploading(docId);
    setError(null);
    try {
      const fd = new FormData();
      fd.set("docId", docId);
      fd.set("file", file);
      const res = await fetch(`${base(token)}/upload`, { method: "POST", body: fd });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Dépôt impossible");
      onView(j as PublicFamilyView);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Dépôt impossible");
    } finally {
      setUploading(null);
    }
  };

  return (
    <section className="space-y-3 rounded-2xl border-2 border-sky-200 bg-sky-50/60 p-4 text-sm">
      <h3 className="font-bold text-sky-950">Pièces à fournir avant le départ</h3>
      {error ? <p role="alert" className="rounded-lg bg-rose-50 p-2 font-semibold text-rose-700">{error}</p> : null}
      {docs.map((d) => {
        const st = view.documents.find((x) => x.id === d.id);
        return (
          <div key={d.id} className="space-y-2 rounded-xl border bg-white p-3">
            <p className="font-semibold">
              {d.label}
              {d.dueDate ? <span className="ml-2 text-xs font-normal text-slate-500">avant le {dateFr(d.dueDate)}</span> : null}
            </p>
            {st?.exempt ? <p className="text-xs font-semibold text-emerald-700">Dispense accordée par l&apos;organisateur.</p> : null}
            {st?.uploaded ? <p className="text-xs font-semibold text-emerald-700">✓ {st.uploaded.fileName}</p> : null}
            <label className="block cursor-pointer rounded-lg border-2 border-dashed border-sky-300 bg-sky-50 px-3 py-3 text-center text-xs font-semibold text-sky-800">
              {uploading === d.id ? "Envoi en cours…" : st?.uploaded ? "Remplacer le fichier" : "Déposer le fichier (PDF ou photo, 8 Mo max.)"}
              <input
                type="file"
                accept="image/*,application/pdf"
                className="sr-only"
                onChange={(e) => {
                  void upload(d.id, e.target.files?.[0]);
                  e.target.value = "";
                }}
              />
            </label>
          </div>
        );
      })}
    </section>
  );
}
