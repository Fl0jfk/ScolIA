"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { useCallback, useEffect, useState } from "react";
import { AwardStatusBadge } from "@/app/components/certificates/CertificatePendingSignaturesPanel";
import { useAppContext } from "@/app/hooks/useAppContext";
import { formatCertificatePersonLabel } from "@/app/lib/certificates-person-label";
import {
  CERTIFICATE_DIRECTION_ROLE_BY_SECTEUR,
  CERTIFICATE_SECTEUR_LABELS,
  type CertificateProgram,
  type StudentAward,
} from "@/app/lib/certificates-types";

type Peer = {
  externalUserId: string;
  displayName: string;
  firstName?: string;
  lastName?: string;
};

export default function CertificateAwardPage() {
  const { programId, awardId } = useParams<{ programId: string; awardId: string }>();
  const { user } = useSessionUser();
  const { data: appContext } = useAppContext();
  const userId = user?.id || "";
  const myRoles = appContext?.session?.intranetRoles ?? [];
  const [award, setAward] = useState<StudentAward | null>(null);
  const [program, setProgram] = useState<CertificateProgram | null>(null);
  const [eligibleIds, setEligibleIds] = useState<string[]>([]);
  const [peers, setPeers] = useState<Peer[]>([]);
  const [lineTitle, setLineTitle] = useState("");
  const [linePeriod, setLinePeriod] = useState("");
  const [lineDescription, setLineDescription] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [hasSignature, setHasSignature] = useState(false);
  const [selectedSignatories, setSelectedSignatories] = useState<string[]>([]);
  const [canManageSignatories, setCanManageSignatories] = useState(false);
  const [directionLabel, setDirectionLabel] = useState("Direction");
  const [addSignatoryId, setAddSignatoryId] = useState("");

  const load = useCallback(async () => {
    const [awardRes, peersRes, sigRes] = await Promise.all([
      fetch(`/api/certificates/awards/${awardId}`, { cache: "no-store" }),
      fetch("/api/certificates/peers", { cache: "no-store" }),
      fetch("/api/certificates/my-signature", { cache: "no-store" }),
    ]);
    const awardData = await awardRes.json();
    const peersData = await peersRes.json();
    const sigData = await sigRes.json();
    if (!awardRes.ok) throw new Error(awardData?.error || "Erreur");
    setAward(awardData.award);
    setProgram(awardData.program);
    setEligibleIds(awardData.eligibleSignatoryIds || []);
    setPeers(peersData.peers || []);
    setHasSignature(Boolean(sigData.hasSignature));
    setCanManageSignatories(Boolean(awardData.permissions?.canManageSignatories));
    setDirectionLabel(awardData.directionLabel || "Direction");
    setSelectedSignatories(
      (awardData.award?.designatedSignatories || []).map((s: { externalUserId: string }) => s.externalUserId),
    );
  }, [awardId]);

  useEffect(() => {
    load().catch(() => undefined);
  }, [load]);

  const isDraft = award?.status === "draft";
  const mySignatory = award?.designatedSignatories.find((s) => s.externalUserId === userId);
  const canSignProf = mySignatory?.status === "pending" && (award?.status === "submitted" || award?.status === "prof_signed");

  const collaboratorNames = (id: string) => {
    const peer = peers.find((p) => p.externalUserId === id);
    if (peer) return formatCertificatePersonLabel(peer);
    return id === program?.ownerId ? program?.ownerName : "Enseignant";
  };

  async function addLine() {
    const title = lineTitle.trim();
    const description = lineDescription.trim();
    if (!title && !description) {
      alert("Indiquez au moins un titre ou une description.");
      return;
    }
    setBusy("line");
    try {
      const res = await fetch(`/api/certificates/awards/${awardId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          addLine: {
            title,
            period: linePeriod.trim() || undefined,
            description,
          },
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setAward(data.award);
      setLineTitle("");
      setLinePeriod("");
      setLineDescription("");
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  async function removeLine(lineId: string) {
    if (!confirm("Supprimer cette ligne ?")) return;
    setBusy(lineId);
    try {
      const res = await fetch(`/api/certificates/awards/${awardId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ removeLineId: lineId }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setAward(data.award);
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  async function removeSignatory(externalUserId: string) {
    if (!confirm("Retirer ce professeur des signataires ?")) return;
    setBusy(`remove-${externalUserId}`);
    try {
      const res = await fetch(`/api/certificates/awards/${awardId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ removeSignatoryId: externalUserId }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setAward(data.award);
      setSelectedSignatories(
        (data.award?.designatedSignatories || []).map((s: { externalUserId: string }) => s.externalUserId),
      );
      await load();
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  async function addSignatory() {
    if (!addSignatoryId) return;
    const merged = [...new Set([...selectedSignatories, addSignatoryId])];
    setBusy("add-signatory");
    try {
      const res = await fetch(`/api/certificates/awards/${awardId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ designatedSignatoryIds: merged }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setAward(data.award);
      setSelectedSignatories(merged);
      setAddSignatoryId("");
      await load();
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  const availableToAddSignatory = eligibleIds.filter(
    (id) => !award?.designatedSignatories.some((s) => s.externalUserId === id),
  );

  const awaitingDirection =
    award?.designatedSignatories.every((s) => s.status === "signed") &&
    !award?.directionSignature &&
    award?.designatedSignatories.length > 0;

  const requiredDirectionRole = award
    ? CERTIFICATE_DIRECTION_ROLE_BY_SECTEUR[award.student.secteur]
    : null;
  const canSignDirection = Boolean(
    awaitingDirection &&
      requiredDirectionRole &&
      myRoles.includes(requiredDirectionRole),
  );

  async function saveSignatories() {
    setBusy("signatories");
    try {
      const res = await fetch(`/api/certificates/awards/${awardId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ designatedSignatoryIds: selectedSignatories }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setAward(data.award);
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  async function submit() {
    if (!confirm("Soumettre cette fiche pour signature ? Les lignes et signataires seront figés.")) return;
    setBusy("submit");
    try {
      const res = await fetch(`/api/certificates/awards/${awardId}/submit`, { method: "POST" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setAward(data.award);
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  async function signProf() {
    if (!hasSignature) {
      alert("Enregistrez d'abord votre paraphe dans « Ma signature ».");
      return;
    }
    setBusy("sign-prof");
    try {
      const res = await fetch(`/api/certificates/awards/${awardId}/sign-prof`, { method: "POST" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setAward(data.award);
      await load();
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  async function signDirection() {
    if (!confirm("Signer en tant que direction ?")) return;
    setBusy("sign-dir");
    try {
      const res = await fetch(`/api/certificates/awards/${awardId}/sign-direction`, { method: "POST" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setAward(data.award);
      await load();
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  async function generatePdf() {
    setBusy("pdf");
    try {
      const res = await fetch(`/api/certificates/awards/${awardId}/generate-pdf`, { method: "POST" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setAward(data.award);
      alert("PDF généré avec succès.");
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  if (!award || !program) {
    return <div className="p-20 text-center font-bold text-slate-500">Chargement…</div>;
  }

  return (
    <div className="px-4 py-6 max-w-3xl mx-auto space-y-6">
      <Link href={`/certificates/${programId}`} className="text-sm font-bold text-indigo-600">
        ← {program.title}
      </Link>

      <div className="rounded-2xl border border-slate-200 bg-white p-5 space-y-2">
        <div className="flex flex-wrap items-center gap-2 justify-between">
          <h1 className="text-2xl font-black text-slate-900">
            {award.student.prenom} {award.student.nom}
          </h1>
          <AwardStatusBadge status={award.status} />
        </div>
        <p className="text-sm text-slate-600">
          {award.student.classe} · {CERTIFICATE_SECTEUR_LABELS[award.student.secteur]} · {award.schoolYear}
        </p>
      </div>

      <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-3">
        <h2 className="text-sm font-black text-slate-900">Lignes du certificat</h2>
        {award.lines.length === 0 && <p className="text-sm text-slate-400">Aucune ligne pour l&apos;instant.</p>}
        <ol className="space-y-2">
          {award.lines.map((line, i) => (
            <li key={line.id} className="flex gap-2 text-sm border-b border-slate-50 pb-3">
              <span className="font-bold text-slate-400 w-6">{i + 1}.</span>
              <div className="flex-1 space-y-1">
                <p className="font-black text-slate-900">
                  {line.title}
                  {line.period && (
                    <span className="font-normal text-slate-500"> — {line.period}</span>
                  )}
                </p>
                <p className="text-slate-700">{line.description}</p>
                <p className="text-xs text-slate-400">
                  {line.addedByName} — {new Date(line.addedAt).toLocaleDateString("fr-FR")}
                </p>
              </div>
              {isDraft && (line.addedBy === userId || program.ownerId === userId) && (
                <button
                  type="button"
                  disabled={busy === line.id}
                  onClick={() => void removeLine(line.id)}
                  className="text-xs text-red-600 font-bold"
                >
                  Suppr.
                </button>
              )}
            </li>
          ))}
        </ol>
        {isDraft && (
          <div className="pt-2 space-y-3 rounded-xl border border-dashed border-slate-200 p-4">
            <label className="block space-y-1">
              <span className="text-xs font-bold text-slate-600">Titre</span>
              <input
                value={lineTitle}
                onChange={(e) => setLineTitle(e.target.value)}
                placeholder="Ex. Séjour surf, Arbitrage scolaire…"
                className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
              />
            </label>
            <label className="block space-y-1">
              <span className="text-xs font-bold text-slate-600">Date ou période</span>
              <input
                value={linePeriod}
                onChange={(e) => setLinePeriod(e.target.value)}
                placeholder="Ex. Mars 2026, Oct.–Déc. 2025…"
                className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
              />
            </label>
            <label className="block space-y-1">
              <span className="text-xs font-bold text-slate-600">Description</span>
              <textarea
                value={lineDescription}
                onChange={(e) => setLineDescription(e.target.value)}
                placeholder="Détaillez la participation, le rôle, les résultats…"
                rows={3}
                className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
              />
            </label>
            <button
              type="button"
              disabled={busy === "line"}
              onClick={() => void addLine()}
              className="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-black text-white disabled:opacity-50"
            >
              Ajouter la ligne
            </button>
          </div>
        )}
      </section>

      <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-3">
        <h2 className="text-sm font-black text-slate-900">Signataires professeurs</h2>
        {isDraft ? (
          <>
            <div className="space-y-2">
              {eligibleIds.map((id) => (
                <label key={id} className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={selectedSignatories.includes(id)}
                    onChange={(e) => {
                      if (e.target.checked) {
                        setSelectedSignatories((prev) => [...prev, id]);
                      } else {
                        setSelectedSignatories((prev) => prev.filter((x) => x !== id));
                      }
                    }}
                  />
                  {collaboratorNames(id)}
                </label>
              ))}
            </div>
            <button
              type="button"
              disabled={busy === "signatories"}
              onClick={() => void saveSignatories()}
              className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-black text-white"
            >
              Enregistrer les signataires
            </button>
          </>
        ) : (
          <ul className="space-y-2 text-sm">
            {award.designatedSignatories.map((s) => (
              <li key={s.externalUserId} className="flex justify-between items-center gap-2">
                <span>{s.name}</span>
                <div className="flex items-center gap-2">
                  <span className={s.status === "signed" ? "text-emerald-700 font-bold" : "text-amber-700"}>
                    {s.status === "signed"
                      ? `Signé le ${s.signedAt ? new Date(s.signedAt).toLocaleDateString("fr-FR") : ""}`
                      : "En attente"}
                  </span>
                  {canManageSignatories && s.status === "pending" && (
                    <button
                      type="button"
                      disabled={busy === `remove-${s.externalUserId}`}
                      onClick={() => void removeSignatory(s.externalUserId)}
                      className="text-xs text-red-600 font-bold"
                    >
                      Retirer
                    </button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        )}
        {canManageSignatories && !isDraft && availableToAddSignatory.length > 0 && (
          <div className="flex gap-2 pt-2 border-t border-slate-100">
            <select
              value={addSignatoryId}
              onChange={(e) => setAddSignatoryId(e.target.value)}
              className="flex-1 rounded-xl border border-slate-200 px-3 py-2 text-sm"
            >
              <option value="">— Ajouter un signataire —</option>
              {availableToAddSignatory.map((id) => (
                <option key={id} value={id}>
                  {collaboratorNames(id)}
                </option>
              ))}
            </select>
            <button
              type="button"
              disabled={!addSignatoryId || busy === "add-signatory"}
              onClick={() => void addSignatory()}
              className="rounded-xl bg-violet-600 px-4 py-2 text-sm font-black text-white disabled:opacity-50"
            >
              Ajouter
            </button>
          </div>
        )}
      </section>

      <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-3">
        <h2 className="text-sm font-black text-slate-900">{directionLabel}</h2>
        {award.directionSignature ? (
          <p className="text-sm text-emerald-700 font-bold">
            Signé par {award.directionSignature.signedByName} le{" "}
            {new Date(award.directionSignature.signedAt).toLocaleDateString("fr-FR")}
          </p>
        ) : awaitingDirection ? (
          <p className="text-sm text-amber-800">
            Tous les professeurs ont signé. En attente de la signature de la {directionLabel.toLowerCase()}.
          </p>
        ) : (
          <p className="text-sm text-slate-500">
            La direction signera une fois que tous les professeurs désignés auront signé.
          </p>
        )}
        {canSignDirection && (
          <button
            type="button"
            disabled={busy === "sign-dir"}
            onClick={() => void signDirection()}
            className="rounded-xl bg-amber-600 px-4 py-2 text-sm font-black text-white disabled:opacity-50"
          >
            Signer en tant que {directionLabel.toLowerCase()}
          </button>
        )}
      </section>

      <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-3">
        <h2 className="text-sm font-black text-slate-900">Actions</h2>
        <div className="flex flex-wrap gap-2">
          {isDraft && (
            <button
              type="button"
              disabled={busy === "submit"}
              onClick={() => void submit()}
              className="rounded-xl bg-violet-600 px-4 py-2 text-sm font-black text-white disabled:opacity-50"
            >
              Soumettre pour signature
            </button>
          )}
          {canSignProf && (
            <button
              type="button"
              disabled={busy === "sign-prof"}
              onClick={() => void signProf()}
              className="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-black text-white disabled:opacity-50"
            >
              Signer (mon paraphe)
            </button>
          )}
          {(award.status === "direction_signed" || award.status === "issued") && (
            <button
              type="button"
              disabled={busy === "pdf"}
              onClick={() => void generatePdf()}
              className="rounded-xl bg-emerald-600 px-4 py-2 text-sm font-black text-white disabled:opacity-50"
            >
              {award.status === "issued" ? "Régénérer le PDF" : "Générer le PDF"}
            </button>
          )}
          {award.status === "issued" && (
            <a
              href={`/api/certificates/awards/${awardId}/pdf`}
              className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-black text-white"
            >
              Télécharger le PDF
            </a>
          )}
        </div>
        {!hasSignature && canSignProf && (
          <p className="text-xs text-amber-800">
            <Link href="/certificates/my-signature" className="underline font-bold">
              Enregistrez votre paraphe
            </Link>{" "}
            avant de signer.
          </p>
        )}
      </section>
    </div>
  );
}
