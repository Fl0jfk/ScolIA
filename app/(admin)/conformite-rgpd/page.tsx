"use client";

import { useCallback, useEffect, useState } from "react";
import dynamic from "next/dynamic";
import RgpdDashboard from "@/app/components/rgpd/RgpdDashboard";
import RgpdDisclaimer from "@/app/components/rgpd/RgpdDisclaimer";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import type { RgpdDocumentContentPreview } from "@/app/lib/rgpd-types";
import type { RgpdActionItem, RgpdDocumentWithMeta } from "@/app/lib/rgpd-scoring";
import type { RgpdComplianceScore, RgpdQuestionnaireAnswers } from "@/app/lib/rgpd-types";
import { DEFAULT_RGPD_ANSWERS } from "@/app/lib/rgpd-types";

type Tab = "dashboard" | "questionnaire" | "documents" | "incidents";

const RgpdQuestionnaireWizard = dynamic(() => import("@/app/components/rgpd/RgpdQuestionnaireWizard"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const RgpdDocumentsPanel = dynamic(() => import("@/app/components/rgpd/RgpdDocumentsPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const RgpdIncidentsPanel = dynamic(() => import("@/app/components/rgpd/RgpdIncidentsPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});

const TABS: { id: Tab; label: string }[] = [
  { id: "dashboard", label: "Tableau de bord" },
  { id: "questionnaire", label: "Questionnaire" },
  { id: "documents", label: "Documents" },
  { id: "incidents", label: "Incidents" },
];

export default function ConformiteRgpdPage() {
  const [tab, setTab] = useState<Tab>("dashboard");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [answers, setAnswers] = useState<RgpdQuestionnaireAnswers>(DEFAULT_RGPD_ANSWERS);
  const [score, setScore] = useState<RgpdComplianceScore | null>(null);
  const [documents, setDocuments] = useState<RgpdDocumentWithMeta[]>([]);
  const [documentPreviews, setDocumentPreviews] = useState<RgpdDocumentContentPreview[]>([]);
  const [actions, setActions] = useState<RgpdActionItem[]>([]);

  const refresh = useCallback(async () => {
    const res = await fetch("/api/rgpd/workspace");
    if (!res.ok) {
      setLoading(false);
      return;
    }
    const data = await res.json();
    setAnswers(data.workspace?.answers ?? DEFAULT_RGPD_ANSWERS);
    setScore(data.score);
    setDocuments(data.documents ?? []);
    setDocumentPreviews(data.documentPreviews ?? []);
    setActions(data.actions ?? []);
    setLoading(false);
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const saveAnswers = async (patch: Partial<RgpdQuestionnaireAnswers>, complete?: boolean) => {
    setSaving(true);
    try {
      const res = await fetch("/api/rgpd/workspace", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          answers: patch,
          markQuestionnaireComplete: complete,
          note: complete ? "Questionnaire terminé" : "Étape questionnaire",
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error);
      setAnswers(data.workspace?.answers ?? answers);
      setScore(data.score);
      setDocuments(data.documents ?? []);
      setDocumentPreviews(data.documentPreviews ?? []);
      setActions(data.actions ?? []);
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="p-8 text-center text-slate-500">Chargement du module RGPD…</div>
    );
  }

  return (
    <ModulePageShell
      maxWidthClass={tab === "documents" ? "max-w-[1500px]" : "max-w-[1280px]"}
      className="space-y-6"
      tourModuleId="conformite-rgpd"
    >
      <div data-tour="rgpd-module">
        <ModulePageHeader
          eyebrow="Établissement"
          title="Conformité RGPD"
          description={
            <>
              <p>
                Questionnaire, documents types, score de conformité et gestion des incidents pour votre
                établissement scolaire.
              </p>
              <RgpdDisclaimer className="mt-2" />
            </>
          }
        />

        <ModuleTabNav tabs={TABS} active={tab} onChange={setTab} className="mb-6" />

        {tab === "dashboard" && score && (
          <RgpdDashboard score={score} actions={actions} documents={documents} />
        )}

        {tab === "questionnaire" && (
          <RgpdQuestionnaireWizard
            answers={answers}
            onChange={(patch) => setAnswers((prev) => ({ ...prev, ...patch }))}
            onSave={saveAnswers}
            saving={saving}
          />
        )}

        {tab === "documents" && (
          <RgpdDocumentsPanel
            documents={documents}
            documentPreviews={documentPreviews}
            onRefresh={refresh}
          />
        )}

        {tab === "incidents" && <RgpdIncidentsPanel />}
      </div>
    </ModulePageShell>
  );
}
