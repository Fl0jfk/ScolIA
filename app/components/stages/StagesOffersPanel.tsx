"use client";

import { useState } from "react";
import type { StageOffer } from "@/app/lib/stage-types";
import { STAGE_OFFER_KIND_LABELS } from "@/app/lib/stage-types";
import type { StagesHubPermissions, StagesOfferForm } from "@/app/components/stages/stages-hub-types";

const LEVELS = ["3e", "2de", "1re", "Tle", "CAP", "BTS"];

function candidatureHref(token: string) {
  return `/stages/candidater?token=${encodeURIComponent(token)}`;
}

function CandidatureLinkBlock({ token }: { token: string }) {
  const path = candidatureHref(token);
  const [copied, setCopied] = useState(false);

  async function copy() {
    const full = typeof window !== "undefined" ? `${window.location.origin}${path}` : path;
    try {
      await navigator.clipboard.writeText(full);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      /* ignore */
    }
  }

  return (
    <div className="mt-3 rounded-lg border border-[#2F6B4A]/20 bg-white/80 p-3 text-xs">
      <p className="font-semibold text-[#1F3D2B]">Lien candidature élève</p>
      <a href={path} className="mt-1 block break-all text-[#2F6B4A] underline" target="_blank" rel="noreferrer">
        {path}
      </a>
      <button
        type="button"
        onClick={() => void copy()}
        className="mt-2 rounded-md border border-stone-300 px-2 py-1 font-semibold text-stone-700 hover:bg-stone-50"
      >
        {copied ? "Copié !" : "Copier le lien complet"}
      </button>
    </div>
  );
}

export default function StagesOffersPanel({
  permissions,
  offers,
  approvedOffers,
  offerForm,
  setOfferForm,
  busy,
  onSubmit,
  onModerate,
  onGoToConventions,
}: {
  permissions: StagesHubPermissions | undefined;
  offers: StageOffer[];
  approvedOffers: StageOffer[];
  offerForm: StagesOfferForm;
  setOfferForm: (next: StagesOfferForm) => void;
  busy: boolean;
  onSubmit: (e: React.FormEvent) => void;
  onModerate: (id: string, status: "approved" | "rejected") => void;
  onGoToConventions: () => void;
}) {
  return (
    <div className="grid gap-8 lg:grid-cols-2">
      {permissions?.canDepositOffer && (
        <form onSubmit={onSubmit} className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm space-y-4">
          <h2 className="text-lg font-bold text-[#1F3D2B]">Déposer une offre</h2>
          <label className="block text-sm">
            Type
            <select
              className="mt-1 w-full rounded-lg border border-stone-300 px-3 py-2"
              value={offerForm.kind}
              onChange={(e) => setOfferForm({ ...offerForm, kind: e.target.value })}
            >
              {Object.entries(STAGE_OFFER_KIND_LABELS).map(([k, v]) => (
                <option key={k} value={k}>
                  {v}
                </option>
              ))}
            </select>
          </label>
          <input
            required
            placeholder="Nom de l'entreprise *"
            className="w-full rounded-lg border border-stone-300 px-3 py-2"
            value={offerForm.companyName}
            onChange={(e) => setOfferForm({ ...offerForm, companyName: e.target.value })}
          />
          <textarea
            required
            placeholder="Description du poste / activité *"
            className="w-full rounded-lg border border-stone-300 px-3 py-2 min-h-[80px]"
            value={offerForm.description}
            onChange={(e) => setOfferForm({ ...offerForm, description: e.target.value })}
          />
          <div className="flex flex-wrap gap-2">
            {LEVELS.map((lv) => (
              <label key={lv} className="text-sm flex items-center gap-1">
                <input
                  type="checkbox"
                  checked={offerForm.targetLevels.includes(lv)}
                  onChange={(e) => {
                    const targetLevels = e.target.checked
                      ? [...offerForm.targetLevels, lv]
                      : offerForm.targetLevels.filter((x) => x !== lv);
                    setOfferForm({ ...offerForm, targetLevels });
                  }}
                />
                {lv}
              </label>
            ))}
          </div>
          <input
            required
            placeholder="Contact (nom) *"
            className="w-full rounded-lg border border-stone-300 px-3 py-2"
            value={offerForm.contactName}
            onChange={(e) => setOfferForm({ ...offerForm, contactName: e.target.value })}
          />
          <input
            required
            type="email"
            placeholder="Contact (e-mail) *"
            className="w-full rounded-lg border border-stone-300 px-3 py-2"
            value={offerForm.contactEmail}
            onChange={(e) => setOfferForm({ ...offerForm, contactEmail: e.target.value })}
          />
          <button
            type="submit"
            disabled={busy}
            className="rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
          >
            Soumettre à la direction
          </button>
        </form>
      )}

      <div className="space-y-4">
        <h2 className="text-lg font-bold text-[#1F3D2B]">Offres</h2>
        {offers.map((o) => (
          <div key={o.id} className="rounded-xl border border-stone-200 bg-white p-4">
            <p className="font-semibold">{o.companyName}</p>
            <p className="text-sm text-stone-600">
              {STAGE_OFFER_KIND_LABELS[o.kind]} · {o.status}
            </p>
            {permissions?.canModerateOffers && o.status === "pending" && (
              <div className="mt-3 flex gap-2">
                <button
                  type="button"
                  onClick={() => onModerate(o.id, "approved")}
                  className="rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-semibold text-white"
                >
                  Valider
                </button>
                <button
                  type="button"
                  onClick={() => onModerate(o.id, "rejected")}
                  className="rounded-lg bg-rose-600 px-3 py-1.5 text-xs font-semibold text-white"
                >
                  Refuser
                </button>
              </div>
            )}
            {o.status === "approved" && o.candidatureToken && (
              <CandidatureLinkBlock token={o.candidatureToken} />
            )}
          </div>
        ))}
        {approvedOffers.length > 0 && (
          <>
            <h3 className="text-sm font-bold text-stone-500 mt-6">Offres validées (réseau)</h3>
            {approvedOffers.map((o) => (
              <div key={o.id} className="rounded-xl border border-emerald-100 bg-emerald-50/50 p-4">
                <p className="font-semibold">{o.companyName}</p>
                <p className="text-sm text-stone-600">{o.description.slice(0, 120)}…</p>
                {o.candidatureToken && <CandidatureLinkBlock token={o.candidatureToken} />}
                <button
                  type="button"
                  className="mt-2 text-xs font-semibold text-[#2F6B4A] underline"
                  onClick={onGoToConventions}
                >
                  Voir les conventions déposées →
                </button>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}
