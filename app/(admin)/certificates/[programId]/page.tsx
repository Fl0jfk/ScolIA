"use client";

import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import { useUser } from "@clerk/nextjs";
import { useCallback, useEffect, useMemo, useState } from "react";
import { AwardStatusBadge } from "@/app/components/certificates/CertificatePendingSignaturesPanel";
import { useAppContext } from "@/app/hooks/useAppContext";
import {
  certificatePersonSearchText,
  formatCertificatePersonLabel,
} from "@/app/lib/certificates-person-label";
import type { CertificateProgram, StudentAward } from "@/app/lib/certificates-types";
import { hasRole } from "@/app/lib/intranet-role-utils";

type Peer = {
  clerkUserId: string;
  displayName: string;
  firstName?: string;
  lastName?: string;
};
type StudentOption = { key: string; label: string; classe: string };

export default function CertificateProgramPage() {
  const { programId } = useParams<{ programId: string }>();
  const { user } = useUser();
  const { data: appContext } = useAppContext();
  const userId = user?.id || "";
  const myRoles = appContext?.session?.intranetRoles ?? [];
  const router = useRouter();
  const [program, setProgram] = useState<CertificateProgram | null>(null);
  const [awards, setAwards] = useState<StudentAward[]>([]);
  const [peers, setPeers] = useState<Peer[]>([]);
  const [loading, setLoading] = useState(true);
  const [q, setQ] = useState("");
  const [classeFilter, setClasseFilter] = useState("");
  const [showAddStudent, setShowAddStudent] = useState(false);
  const [students, setStudents] = useState<StudentOption[]>([]);
  const [studentQ, setStudentQ] = useState("");
  const [selectedStudentKey, setSelectedStudentKey] = useState("");
  const [selectedSignatories, setSelectedSignatories] = useState<string[]>([]);
  const [addBusy, setAddBusy] = useState(false);
  const [titleDraft, setTitleDraft] = useState("");
  const [savingTitle, setSavingTitle] = useState(false);
  const [collaboratorQ, setCollaboratorQ] = useState("");
  const [selectedCollaboratorId, setSelectedCollaboratorId] = useState("");
  const [bulkBusy, setBulkBusy] = useState<string | null>(null);
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const [deleteConfirmInput, setDeleteConfirmInput] = useState("");
  const [deleteBusy, setDeleteBusy] = useState(false);

  const DELETE_CONFIRM_WORD = "supprimer";

  const load = useCallback(async () => {
    const res = await fetch(`/api/certificates/programs/${programId}`, { cache: "no-store" });
    const data = await res.json();
    if (!res.ok) throw new Error(data?.error || "Erreur");
    setProgram(data.program);
    setAwards(data.awards || []);
    setTitleDraft(data.program?.title || "");
  }, [programId]);

  useEffect(() => {
    Promise.all([
      load().catch(() => undefined),
      fetch("/api/certificates/peers", { cache: "no-store" })
        .then((r) => r.json())
        .then((j) => setPeers(j.peers || []))
        .catch(() => setPeers([])),
    ]).finally(() => setLoading(false));
  }, [load]);

  const isOwner = program?.ownerId === userId;
  const canUseOcrBulk = hasRole(myRoles, "administratif") || appContext?.session?.isGlobalAdmin === true;

  const collaborators = useMemo(() => {
    if (!program) return [];
    const ids = [program.ownerId, ...program.collaboratorIds];
    return ids
      .filter((id, i, arr) => id && arr.indexOf(id) === i)
      .map((id) => {
        const peer = peers.find((p) => p.clerkUserId === id);
        return {
          clerkUserId: id,
          displayName: peer
            ? formatCertificatePersonLabel(peer)
            : id === program.ownerId
              ? program.ownerName
              : "Collaborateur",
          isOwner: id === program.ownerId,
        };
      });
  }, [program, peers]);

  const classes = useMemo(
    () => [...new Set(awards.map((a) => a.student.classe).filter(Boolean))].sort((a, b) => a.localeCompare(b, "fr")),
    [awards],
  );

  const filteredAwards = useMemo(() => {
    const qq = q.trim().toLowerCase();
    return awards.filter((a) => {
      if (classeFilter && a.student.classe !== classeFilter) return false;
      if (!qq) return true;
      const name = `${a.student.prenom} ${a.student.nom}`.toLowerCase();
      return name.includes(qq) || a.student.classe.toLowerCase().includes(qq);
    });
  }, [awards, q, classeFilter]);

  const availableCollaborators = useMemo(() => {
    if (!program) return [] as Peer[];
    const excluded = new Set([program.ownerId, ...program.collaboratorIds]);
    const qq = collaboratorQ.trim().toLowerCase();
    return peers
      .filter((p) => !excluded.has(p.clerkUserId))
      .filter((p) => {
        if (!qq) return true;
        return certificatePersonSearchText(p).includes(qq);
      });
  }, [peers, program, collaboratorQ]);

  async function saveTitle() {
    if (!program) return;
    setSavingTitle(true);
    try {
      const res = await fetch(`/api/certificates/programs/${program.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ title: titleDraft }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setProgram(data.program);
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSavingTitle(false);
    }
  }

  async function toggleCollaborator(clerkUserId: string, add: boolean) {
    if (!program) return;
    const res = await fetch(`/api/certificates/programs/${program.id}/collaborators`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ collaboratorId: clerkUserId, action: add ? "add" : "remove" }),
    });
    const data = await res.json();
    if (!res.ok) {
      alert(data?.error || "Erreur");
      return;
    }
    setProgram(data.program);
  }

  async function addSelectedCollaborator() {
    if (!selectedCollaboratorId) return;
    await toggleCollaborator(selectedCollaboratorId, true);
    setSelectedCollaboratorId("");
  }

  async function openAddStudent() {
    setShowAddStudent(true);
    setSelectedStudentKey("");
    setSelectedSignatories([]);
    const res = await fetch("/api/certificates/students", { cache: "no-store" });
    const data = await res.json();
    setStudents(
      (data.students || []).map((s: { key: string; label: string; classe: string }) => ({
        key: s.key,
        label: s.label,
        classe: s.classe,
      })),
    );
  }

  async function searchStudents() {
    const params = new URLSearchParams();
    if (studentQ.trim()) params.set("q", studentQ.trim());
    const res = await fetch(`/api/certificates/students?${params}`, { cache: "no-store" });
    const data = await res.json();
    setStudents(
      (data.students || []).map((s: { key: string; label: string; classe: string }) => ({
        key: s.key,
        label: s.label,
        classe: s.classe,
      })),
    );
  }

  async function addStudent() {
    if (!selectedStudentKey || !selectedSignatories.length) {
      alert("Choisissez un élève et au moins un signataire.");
      return;
    }
    setAddBusy(true);
    try {
      const res = await fetch("/api/certificates/awards", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          programId,
          studentKey: selectedStudentKey,
          designatedSignatoryIds: selectedSignatories,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setShowAddStudent(false);
      await load();
      router.push(`/certificates/${programId}/${data.award.id}`);
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setAddBusy(false);
    }
  }

  async function runBulkAction(action: "sign-prof-all" | "generate-pdf-all" | "send-ocr-all") {
    setBulkBusy(action);
    try {
      const res = await fetch(`/api/certificates/programs/${programId}/bulk`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      await load();
      const suffix = data.errors?.length
        ? `\n\nErreurs (${data.errors.length}) :\n- ${data.errors.slice(0, 5).join("\n- ")}`
        : "";
      const doneLabel = action === "send-ocr-all" ? "envoyés en OCR" : "traités";
      alert(
        `${data.done}/${data.eligible} ${doneLabel}.${suffix}${
          data.jobId ? `\n\nJob OCR : ${data.jobId}` : ""
        }`,
      );
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBulkBusy(null);
    }
  }

  function downloadAllCertificatesZip() {
    window.open(`/api/certificates/programs/${programId}/bulk`, "_blank");
  }

  async function deleteProgram() {
    if (deleteConfirmInput.trim().toLowerCase() !== DELETE_CONFIRM_WORD) return;
    setDeleteBusy(true);
    try {
      const res = await fetch(`/api/certificates/programs/${programId}`, {
        method: "DELETE",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ confirmation: deleteConfirmInput.trim() }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      router.push("/certificates");
      router.refresh();
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setDeleteBusy(false);
    }
  }

  function openDeleteModal() {
    setDeleteConfirmInput("");
    setShowDeleteModal(true);
  }

  if (loading) return <div className="p-20 text-center font-bold">Chargement…</div>;
  if (!program) return <div className="p-20 text-center text-red-600">Parcours introuvable.</div>;

  return (
    <div className="px-4 py-6 max-w-[1280px] mx-auto space-y-6">
      <Link href="/certificates" className="text-sm font-bold text-indigo-600">
        ← Tous les parcours
      </Link>

      <div className="rounded-2xl border border-slate-200 bg-white p-5 space-y-3">
        <div className="flex flex-wrap gap-2 items-end">
          <label className="flex-1 min-w-[200px] space-y-1">
            <span className="text-xs font-bold text-slate-500">Titre du parcours</span>
            <input
              value={titleDraft}
              onChange={(e) => setTitleDraft(e.target.value)}
              className="w-full rounded-xl border border-slate-200 px-3 py-2 font-black text-lg"
            />
          </label>
          <button
            type="button"
            disabled={savingTitle}
            onClick={() => void saveTitle()}
            className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-black text-white"
          >
            Enregistrer
          </button>
        </div>
        <p className="text-sm text-slate-500">
          Année {program.schoolYear} · Créateur : {program.ownerName}
        </p>
        {isOwner && (
          <div className="pt-2 border-t border-slate-100">
            <button
              type="button"
              onClick={openDeleteModal}
              className="rounded-xl border border-red-200 bg-red-50 px-4 py-2 text-sm font-black text-red-700 hover:bg-red-100"
            >
              Supprimer le parcours
            </button>
          </div>
        )}
      </div>

      <div className="rounded-2xl border border-slate-200 bg-white p-5 space-y-3">
        <p className="text-sm font-black text-slate-900">Collaborateurs du parcours</p>
        <ul className="space-y-1 text-sm">
          {collaborators.map((c) => (
            <li key={c.clerkUserId} className="flex justify-between items-center">
              <span>
                {c.displayName} {c.isOwner && <span className="text-xs text-slate-400">(créateur)</span>}
              </span>
              {!c.isOwner && isOwner && (
                <button
                  type="button"
                  onClick={() => void toggleCollaborator(c.clerkUserId, false)}
                  className="text-xs text-red-600 font-bold"
                >
                  Retirer
                </button>
              )}
            </li>
          ))}
        </ul>
        {isOwner && (
          <div className="pt-2 space-y-2">
            <p className="text-xs font-bold text-slate-500">Ajouter un collaborateur (annuaire Clerk complet)</p>
            <input
              type="text"
              value={collaboratorQ}
              onChange={(e) => setCollaboratorQ(e.target.value)}
              placeholder="Rechercher par nom, prénom…"
              className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
            />
            <div className="flex gap-2">
              <select
                value={selectedCollaboratorId}
                onChange={(e) => setSelectedCollaboratorId(e.target.value)}
                className="flex-1 rounded-xl border border-slate-200 px-3 py-2 text-sm"
              >
                <option value="">— Sélectionner une personne —</option>
                {availableCollaborators.map((p) => (
                  <option key={p.clerkUserId} value={p.clerkUserId}>
                    {formatCertificatePersonLabel(p)}
                  </option>
                ))}
              </select>
              <button
                type="button"
                disabled={!selectedCollaboratorId}
                onClick={() => void addSelectedCollaborator()}
                className="rounded-xl bg-violet-600 px-4 py-2 text-sm font-black text-white disabled:opacity-50"
              >
                Ajouter
              </button>
            </div>
            <p className="text-xs text-slate-400">Tri : nom de famille A → Z</p>
          </div>
        )}
      </div>

      <div className="flex flex-wrap gap-2 items-center justify-between">
        <h2 className="text-xl font-black text-slate-900">Élèves ({awards.length})</h2>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            disabled={bulkBusy === "sign-prof-all"}
            onClick={() => void runBulkAction("sign-prof-all")}
            className="rounded-xl bg-indigo-700 px-4 py-2 text-sm font-black text-white disabled:opacity-50"
          >
            Signer mes élèves (en masse)
          </button>
          <button
            type="button"
            disabled={bulkBusy === "generate-pdf-all"}
            onClick={() => void runBulkAction("generate-pdf-all")}
            className="rounded-xl bg-emerald-700 px-4 py-2 text-sm font-black text-white disabled:opacity-50"
          >
            Générer tous les PDF
          </button>
          <button
            type="button"
            onClick={downloadAllCertificatesZip}
            className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-black text-white"
          >
            Télécharger tous les certificats (ZIP)
          </button>
          {canUseOcrBulk && (
            <button
              type="button"
              disabled={bulkBusy === "send-ocr-all"}
              onClick={() => void runBulkAction("send-ocr-all")}
              className="rounded-xl bg-amber-600 px-4 py-2 text-sm font-black text-white disabled:opacity-50"
            >
              Envoyer tous les PDF en IA OCR
            </button>
          )}
          <button
            type="button"
            onClick={() => void openAddStudent()}
            className="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-black text-white"
          >
            Ajouter un élève
          </button>
        </div>
      </div>

      <div className="flex flex-wrap gap-2">
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="Rechercher un élève…"
          className="rounded-xl border border-slate-200 px-3 py-2 text-sm flex-1 min-w-[180px]"
        />
        <select
          value={classeFilter}
          onChange={(e) => setClasseFilter(e.target.value)}
          className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
        >
          <option value="">Toutes les classes</option>
          {classes.map((c) => (
            <option key={c} value={c}>
              {c}
            </option>
          ))}
        </select>
      </div>

      <div className="overflow-x-auto rounded-2xl border border-slate-200 bg-white">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-slate-100 text-left text-xs text-slate-500">
              <th className="p-3 font-bold">Élève</th>
              <th className="p-3 font-bold">Classe</th>
              <th className="p-3 font-bold">Lignes</th>
              <th className="p-3 font-bold">Signataires</th>
              <th className="p-3 font-bold">Statut</th>
            </tr>
          </thead>
          <tbody>
            {filteredAwards.map((a) => {
              const signed = a.designatedSignatories.filter((s) => s.status === "signed").length;
              const total = a.designatedSignatories.length;
              return (
                <tr key={a.id} className="border-b border-slate-50 hover:bg-slate-50">
                  <td className="p-3">
                    <Link
                      href={`/certificates/${programId}/${a.id}`}
                      className="font-bold text-indigo-700 hover:underline"
                    >
                      {a.student.prenom} {a.student.nom}
                    </Link>
                  </td>
                  <td className="p-3">{a.student.classe || "—"}</td>
                  <td className="p-3">{a.lines.length}</td>
                  <td className="p-3 text-xs">
                    {signed}/{total} signés
                  </td>
                  <td className="p-3">
                    <AwardStatusBadge status={a.status} />
                  </td>
                </tr>
              );
            })}
            {filteredAwards.length === 0 && (
              <tr>
                <td colSpan={5} className="p-8 text-center text-slate-400">
                  Aucun élève pour l&apos;instant.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {showDeleteModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
          <div className="w-full max-w-lg rounded-2xl bg-white p-5 space-y-4">
            <p className="text-lg font-black text-red-700">Supprimer ce parcours ?</p>
            <div className="rounded-xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-950 space-y-2">
              <p className="font-black">Attention — action irréversible</p>
              <p>
                Si vous supprimez ce parcours, <strong>toutes les données</strong> associées seront définitivement
                effacées : fiches élèves, signatures, liens de vérification, etc.
              </p>
              <p>
                Les <strong>PDF non téléchargés</strong> (ou non enregistrés ailleurs) seront également supprimés et
                ne pourront pas être récupérés.
              </p>
            </div>
            <label className="block space-y-1">
              <span className="text-sm font-bold text-slate-700">
                Pour confirmer, tapez <span className="font-mono text-red-700">{DELETE_CONFIRM_WORD}</span> ci-dessous
              </span>
              <input
                value={deleteConfirmInput}
                onChange={(e) => setDeleteConfirmInput(e.target.value)}
                placeholder={DELETE_CONFIRM_WORD}
                className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-mono"
                autoComplete="off"
                spellCheck={false}
              />
            </label>
            <div className="flex gap-2 justify-end">
              <button
                type="button"
                onClick={() => setShowDeleteModal(false)}
                disabled={deleteBusy}
                className="rounded-xl bg-slate-100 px-4 py-2 text-sm font-bold disabled:opacity-50"
              >
                Annuler
              </button>
              <button
                type="button"
                disabled={deleteBusy || deleteConfirmInput.trim().toLowerCase() !== DELETE_CONFIRM_WORD}
                onClick={() => void deleteProgram()}
                className="rounded-xl bg-red-600 px-4 py-2 text-sm font-black text-white disabled:opacity-50"
              >
                {deleteBusy ? "Suppression…" : "Supprimer définitivement"}
              </button>
            </div>
          </div>
        </div>
      )}

      {showAddStudent && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
          <div className="w-full max-w-lg rounded-2xl bg-white p-5 space-y-4 max-h-[90vh] overflow-y-auto">
            <p className="text-lg font-black">Ajouter un élève</p>
            <div className="flex gap-2">
              <input
                value={studentQ}
                onChange={(e) => setStudentQ(e.target.value)}
                placeholder="Rechercher…"
                className="flex-1 rounded-xl border border-slate-200 px-3 py-2 text-sm"
              />
              <button
                type="button"
                onClick={() => void searchStudents()}
                className="rounded-xl bg-slate-100 px-3 py-2 text-sm font-bold"
              >
                Chercher
              </button>
            </div>
            <select
              value={selectedStudentKey}
              onChange={(e) => setSelectedStudentKey(e.target.value)}
              className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
            >
              <option value="">— Choisir un élève —</option>
              {students.map((s) => (
                <option key={s.key} value={s.key}>
                  {s.label}
                </option>
              ))}
            </select>
            <div>
              <p className="text-sm font-bold text-slate-700 mb-2">Profs signataires pour cette fiche</p>
              <div className="space-y-2 max-h-40 overflow-y-auto">
                {collaborators.map((c) => (
                  <label key={c.clerkUserId} className="flex items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={selectedSignatories.includes(c.clerkUserId)}
                      onChange={(e) => {
                        if (e.target.checked) {
                          setSelectedSignatories((prev) => [...prev, c.clerkUserId]);
                        } else {
                          setSelectedSignatories((prev) => prev.filter((id) => id !== c.clerkUserId));
                        }
                      }}
                    />
                    {c.displayName}
                  </label>
                ))}
              </div>
            </div>
            <div className="flex gap-2 justify-end">
              <button
                type="button"
                onClick={() => setShowAddStudent(false)}
                className="rounded-xl bg-slate-100 px-4 py-2 text-sm font-bold"
              >
                Annuler
              </button>
              <button
                type="button"
                disabled={addBusy}
                onClick={() => void addStudent()}
                className="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-black text-white disabled:opacity-50"
              >
                {addBusy ? "Ajout…" : "Créer la fiche"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
