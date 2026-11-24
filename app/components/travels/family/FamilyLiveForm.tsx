"use client";

/**
 * Formulaire réel de la page des familles (mode « live ») : réponses enregistrées au fil de la saisie,
 * dépôt de pièces, code par e-mail puis signature. Toute la validation fait foi CÔTÉ SERVEUR.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import type { PublicFamilyView } from "@/app/lib/travels-v2-registration";
import { MEAL_TYPE_VALUES, MEAL_TYPE_LABELS, type MealType } from "@/app/lib/travels-v2-registration-types";
import { FAMILY_PAGE_TEMPLATE } from "@/app/lib/travels-family-page-template";
import { dateFr } from "@/app/lib/travels-family-page";

type Props = {
  token: string;
  view: PublicFamilyView;
  onView: (v: PublicFamilyView) => void;
};

const base = (token: string) => `/api/voyages/famille/${encodeURIComponent(token)}`;

function fmtSize(n: number): string {
  return n > 1024 * 1024 ? `${(n / 1024 / 1024).toFixed(1)} Mo` : `${Math.max(1, Math.round(n / 1024))} Ko`;
}

/* ─── Zone de tracé de la signature (souris et doigt) ─── */

function SignaturePad({ onChange, disabled }: { onChange: (png: string | null) => void; disabled: boolean }) {
  const ref = useRef<HTMLCanvasElement | null>(null);
  const drawing = useRef(false);
  const inked = useRef(false);

  const pos = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const c = ref.current!;
    const r = c.getBoundingClientRect();
    return { x: ((e.clientX - r.left) / r.width) * c.width, y: ((e.clientY - r.top) / r.height) * c.height };
  };
  const start = (e: React.PointerEvent<HTMLCanvasElement>) => {
    if (disabled) return;
    const c = ref.current!;
    c.setPointerCapture(e.pointerId);
    const ctx = c.getContext("2d")!;
    const { x, y } = pos(e);
    ctx.lineWidth = 3;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.strokeStyle = "#0f172a";
    ctx.beginPath();
    ctx.moveTo(x, y);
    drawing.current = true;
  };
  const move = (e: React.PointerEvent<HTMLCanvasElement>) => {
    if (!drawing.current || disabled) return;
    const ctx = ref.current!.getContext("2d")!;
    const { x, y } = pos(e);
    ctx.lineTo(x, y);
    ctx.stroke();
    inked.current = true;
  };
  const end = () => {
    if (!drawing.current) return;
    drawing.current = false;
    if (inked.current) onChange(ref.current!.toDataURL("image/png"));
  };
  const clear = () => {
    const c = ref.current!;
    c.getContext("2d")!.clearRect(0, 0, c.width, c.height);
    inked.current = false;
    onChange(null);
  };

  return (
    <div className="space-y-1">
      <canvas
        ref={ref}
        width={600}
        height={200}
        aria-label="Zone de signature : tracez avec la souris ou le doigt"
        className="h-28 w-full touch-none rounded-lg border-2 border-dashed border-slate-300 bg-white sm:h-36"
        onPointerDown={start}
        onPointerMove={move}
        onPointerUp={end}
        onPointerCancel={end}
        onPointerLeave={end}
      />
      <button type="button" onClick={clear} className="text-xs font-semibold text-indigo-600 underline" disabled={disabled}>
        Effacer et recommencer
      </button>
    </div>
  );
}

export function FamilyLiveForm({ token, view, onView }: Props) {
  const h = FAMILY_PAGE_TEMPLATE.headings;
  const m = view.model;
  const open = view.window.open;
  const a = view.answers;
  const [phone, setPhone] = useState(a.phone || "");
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [uploading, setUploading] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [signerName, setSignerName] = useState("");
  const [code, setCode] = useState("");
  const [png, setPng] = useState<string | null>(null);
  const [cooldownUntil, setCooldownUntil] = useState(0);
  const [, forceTick] = useState(0);
  const phoneTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (cooldownUntil <= Date.now()) return;
    const t = setInterval(() => forceTick((x) => x + 1), 1000);
    return () => clearInterval(t);
  }, [cooldownUntil]);

  /**
   * Enregistrements en file : jamais deux PATCH en vol en même temps (le serveur les appliquerait dans un ordre
   * imprévisible). Les modifications arrivées pendant une requête sont fusionnées champ par champ (les repas par jour)
   * et envoyées ensemble à la fin ; chaque requête ne contient que les champs modifiés.
   */
  const pendingRef = useRef<Record<string, unknown> | null>(null);
  const flushingRef = useRef(false);

  const patch = useCallback(
    async (body: Record<string, unknown>) => {
      const prev = pendingRef.current;
      const prevMeals = (prev?.meals as Record<string, unknown> | undefined) || {};
      const nextMeals = (body.meals as Record<string, unknown> | undefined) || {};
      pendingRef.current = {
        ...(prev || {}),
        ...body,
        ...(prev?.meals || body.meals ? { meals: { ...prevMeals, ...nextMeals } } : {}),
      };
      if (flushingRef.current) return;
      flushingRef.current = true;
      setSaving(true);
      setError(null);
      try {
        while (pendingRef.current) {
          const toSend = pendingRef.current;
          pendingRef.current = null;
          const res = await fetch(base(token), {
            method: "PATCH",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(toSend),
          });
          const j = await res.json().catch(() => ({}));
          if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
          onView(j as PublicFamilyView);
        }
      } catch (e) {
        pendingRef.current = null;
        setError(e instanceof Error ? e.message : "Enregistrement impossible");
      } finally {
        flushingRef.current = false;
        setSaving(false);
      }
    },
    [token, onView],
  );

  const onPhone = (value: string) => {
    setPhone(value);
    if (phoneTimer.current) clearTimeout(phoneTimer.current);
    phoneTimer.current = setTimeout(() => {
      if (value.trim() === "" || /^[+0-9 ().-]{6,20}$/.test(value.trim())) void patch({ phone: value.trim() });
    }, 700);
  };

  const upload = async (docId: string, files: File[]) => {
    const file = files[0];
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

  const removeDoc = async (docId: string) => {
    setError(null);
    const res = await fetch(`${base(token)}/upload`, {
      method: "DELETE",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ docId }),
    });
    const j = await res.json().catch(() => ({}));
    if (!res.ok) setError(j.error || "Suppression impossible");
    else onView(j as PublicFamilyView);
  };

  const sendCode = async () => {
    setBusy(true);
    setError(null);
    setInfo(null);
    try {
      const res = await fetch(`${base(token)}/code`, { method: "POST" });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) {
        if (j.retryAfterSec) setCooldownUntil(Date.now() + j.retryAfterSec * 1000);
        throw new Error(j.error || "Envoi du code impossible");
      }
      setCooldownUntil(Date.now() + 30_000);
      setInfo(`Un code à 6 chiffres vient d'être envoyé à ${j.maskedEmail}. Il est valable 10 minutes.`);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Envoi du code impossible");
    } finally {
      setBusy(false);
    }
  };

  const sign = async () => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`${base(token)}/sign`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ signerName, code, ...(a.participates === "oui" && png ? { signaturePng: png } : {}) }),
      });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) {
        throw new Error(
          j.error + (typeof j.attemptsLeft === "number" ? ` (${j.attemptsLeft} essai(s) restant(s))` : ""),
        );
      }
      onView(j.view as PublicFamilyView);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Signature impossible");
    } finally {
      setBusy(false);
    }
  };

  const cooldown = Math.max(0, Math.ceil((cooldownUntil - Date.now()) / 1000));
  const decision = a.participates;
  const canSubmit =
    open &&
    view.problems.length === 0 &&
    signerName.trim().length >= 2 &&
    /^\d{6}$/.test(code) &&
    (decision === "non" || Boolean(png)) &&
    !busy &&
    !saving;
  const inputCls = "rounded-lg border bg-white p-2 text-sm disabled:bg-slate-100 disabled:text-slate-400";

  if (view.siblingDecision === "oui" && !view.locked) {
    return (
      <section className="rounded-2xl border-2 border-emerald-200 bg-emerald-50 p-4 text-sm text-emerald-900">
        L&apos;autre responsable a déjà inscrit {view.studentName}. Aucune action n&apos;est nécessaire de votre part.
      </section>
    );
  }

  return (
    <section className="space-y-5 rounded-2xl border-2 border-indigo-100 bg-indigo-50/40 p-4">
      <div className="flex items-center justify-between">
        <h3 className="text-base font-bold text-indigo-900">Votre réponse pour {view.studentName}</h3>
        <span className="text-xs text-slate-500" aria-live="polite">
          {saving ? "Enregistrement…" : "Enregistré automatiquement"}
        </span>
      </div>

      {error ? <p role="alert" className="rounded-lg bg-rose-50 p-2 text-sm font-semibold text-rose-700">{error}</p> : null}

      <fieldset disabled={!open} className="space-y-2">
        <legend className="text-sm font-bold">Mon enfant {view.studentName} participe au voyage</legend>
        <div className="flex gap-4 text-sm">
          {(["oui", "non"] as const).map((v) => (
            <label key={v} className="flex items-center gap-2">
              <input type="radio" name="participe" value={v} checked={a.participates === v} onChange={() => void patch({ participates: v })} />
              {v === "oui" ? "Oui" : "Non"}
            </label>
          ))}
        </div>
      </fieldset>

      {a.participates === "oui" ? (
        <>
          {m.picnicDays.length > 0 ? (
            <fieldset disabled={!open} className="space-y-2">
              <legend className="text-sm font-bold">Panier repas fourni par l’établissement</legend>
              <p className="text-xs text-slate-600">{m.mealsRule[m.mealsRule.length - 1]}</p>
              <div className="space-y-2">
                {m.picnicDays.map((d) => {
                  const c = a.meals?.[d.key];
                  return (
                    <div key={d.key} className="flex flex-col gap-2 rounded-lg border bg-white p-2 text-sm sm:flex-row sm:items-center sm:gap-4">
                      <span className="w-28 font-semibold">{d.label}</span>
                      <div className="flex flex-wrap gap-4">
                        {(["oui", "non"] as const).map((v) => (
                          <label key={v} className="flex items-center gap-2">
                            <input
                              type="radio"
                              name={`panier-${d.key}`}
                              checked={c?.wants === v}
                              onChange={() => void patch({ meals: { [d.key]: v === "oui" ? { wants: "oui", type: c?.type ?? "classique" } : { wants: "non" } } })}
                            />
                            {v === "oui" ? "Oui" : "Non, nous fournissons le repas"}
                          </label>
                        ))}
                      </div>
                      {c?.wants === "oui" ? (
                        <select
                          value={c.type ?? "classique"}
                          aria-label={`Type de panier — ${d.label}`}
                          className={inputCls}
                          onChange={(e) => void patch({ meals: { [d.key]: { wants: "oui", type: e.target.value as MealType } } })}
                        >
                          {MEAL_TYPE_VALUES.map((t) => (
                            <option key={t} value={t}>
                              {MEAL_TYPE_LABELS[t]}
                            </option>
                          ))}
                        </select>
                      ) : null}
                    </div>
                  );
                })}
              </div>
            </fieldset>
          ) : null}

          <fieldset disabled={!open} className="space-y-1">
            <legend className="text-sm font-bold">Téléphone joignable pendant le voyage</legend>
            <input
              type="tel"
              inputMode="tel"
              autoComplete="tel"
              maxLength={20}
              value={phone}
              onChange={(e) => onPhone(e.target.value)}
              placeholder="06 00 00 00 00"
              className={`w-full max-w-xs ${inputCls}`}
            />
          </fieldset>

          <div className="space-y-3">
            <h4 className="text-sm font-bold">{h.documents}</h4>
            <p className="text-xs text-slate-500">PDF ou photo, 8 Mo maximum. Aucun document médical : la fiche sanitaire est déjà gérée par l&apos;établissement.</p>
            {m.documents.length === 0 ? <p className="text-xs text-slate-500">Aucune pièce demandée pour ce voyage.</p> : null}
            {m.documents.map((d) => {
              const st = view.documents.find((x) => x.id === d.id);
              return (
                <div key={d.id} className="space-y-2 rounded-xl border bg-white p-3">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="text-sm font-semibold">{d.label}</span>
                    <span
                      className={`rounded-full px-2 py-0.5 text-[11px] font-bold ${
                        d.kind === "avant_signature" ? "bg-rose-100 text-rose-800" : d.kind === "avant_depart" ? "bg-sky-100 text-sky-800" : "bg-slate-100 text-slate-600"
                      }`}
                    >
                      {d.kindLabel}
                      {d.dueDate ? ` — avant le ${dateFr(d.dueDate)}` : ""}
                    </span>
                  </div>
                  {st?.exempt ? <p className="text-xs font-semibold text-emerald-700">Dispense accordée par l&apos;organisateur.</p> : null}
                  {st?.uploaded ? (
                    <div className="flex flex-wrap items-center gap-2 text-sm">
                      <span className="font-semibold text-emerald-700">✓ {st.uploaded.fileName}</span>
                      <span className="text-xs text-slate-500">({fmtSize(st.uploaded.size)})</span>
                      {open ? (
                        <button type="button" onClick={() => void removeDoc(d.id)} className="text-xs font-semibold text-rose-600 underline">
                          Retirer
                        </button>
                      ) : null}
                    </div>
                  ) : null}
                  {open ? (
                    <label
                      className="block cursor-pointer rounded-lg border-2 border-dashed border-indigo-300 bg-indigo-50 px-3 py-4 text-center text-xs font-semibold text-indigo-700"
                      onDragOver={(e) => e.preventDefault()}
                      onDrop={(e) => {
                        e.preventDefault();
                        void upload(d.id, Array.from(e.dataTransfer.files));
                      }}
                    >
                      {uploading === d.id ? "Envoi en cours…" : st?.uploaded ? "Remplacer : glisser-déposer ou choisir un fichier" : "Glisser-déposer ou prendre une photo"}
                      <input
                        type="file"
                        accept="image/*,application/pdf"
                        className="sr-only"
                        onChange={(e) => {
                          void upload(d.id, Array.from(e.target.files || []));
                          e.target.value = "";
                        }}
                      />
                    </label>
                  ) : null}
                </div>
              );
            })}
          </div>
        </>
      ) : null}

      {/* Signature */}
      {a.participates ? (
        <fieldset disabled={!open} className="space-y-3 rounded-xl border bg-white p-3">
          <legend className="px-1 text-sm font-bold">Signature</legend>
          {view.problems.length > 0 ? (
            <ul className="list-inside list-disc rounded-lg bg-amber-50 p-2 text-xs font-semibold text-amber-900">
              {view.problems.map((p) => (
                <li key={p}>{p}</li>
              ))}
            </ul>
          ) : (
            <p className="text-xs text-slate-600">
              {decision === "oui"
                ? "En signant, vous inscrivez votre enfant et vous vous engagez à régler la participation financière indiquée sur cette page."
                : "En signant, vous confirmez que votre enfant ne participe pas au voyage."}{" "}
              Après signature, votre réponse ne peut plus être modifiée.
            </p>
          )}
          <div className="flex flex-wrap items-center gap-2">
            <button
              type="button"
              onClick={() => void sendCode()}
              disabled={busy || cooldown > 0 || view.problems.length > 0}
              className="rounded-lg bg-slate-200 px-3 py-2 text-xs font-bold text-slate-700 disabled:opacity-50"
            >
              {cooldown > 0 ? `Nouveau code dans ${cooldown} s` : "Recevoir mon code"}
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
          <input
            value={signerName}
            onChange={(e) => setSignerName(e.target.value)}
            maxLength={100}
            autoComplete="name"
            placeholder="Nom et prénom du responsable"
            className={`w-full ${inputCls}`}
          />
          {decision === "oui" ? <SignaturePad onChange={setPng} disabled={!open} /> : null}
          <button
            type="button"
            onClick={() => void sign()}
            disabled={!canSubmit}
            className="w-full rounded-xl bg-indigo-600 px-4 py-3 text-sm font-bold text-white disabled:opacity-40 sm:w-auto"
          >
            {busy ? "Un instant…" : decision === "oui" ? "Signer et inscrire mon enfant" : "Confirmer que mon enfant ne participe pas"}
          </button>
        </fieldset>
      ) : null}
    </section>
  );
}
