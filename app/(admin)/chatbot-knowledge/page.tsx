"use client";

import { useState } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";

type IngestUpdate = {
  domain: string;
  file?: string;
  title: string;
  contentPreview?: string;
};

export default function ChatbotKnowledgePage() {
  const [text, setText] = useState("");
  const [source, setSource] = useState("");
  const [audience, setAudience] = useState<"public" | "private" | "both">("both");
  const [status, setStatus] = useState("");
  const [loading, setLoading] = useState(false);
  const [pdfFile, setPdfFile] = useState<File | null>(null);
  const [lastUpdates, setLastUpdates] = useState<IngestUpdate[]>([]);
  const injectText = async (payloadText: string, payloadSource?: string) => {
    const res = await fetch("/api/chatbot/ingest", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        text: payloadText,
        source: payloadSource || source || "Saisie manuelle",
        audience,
      }),
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data.error || "Erreur inconnue");
    setLastUpdates(Array.isArray(data.updates) ? data.updates : []);
    return data;
  };
  const submitManual = async () => {
    if (!text.trim()) return;
    setLoading(true);
    setStatus("Injection en cours...");
    try {
      const data = await injectText(text.trim());
      const count = Array.isArray(data.updates) ? data.updates.length : 0;
      setStatus(`OK: ${count} entrée(s) créée(s)`);
      setText("");
    } catch (e) {
      setStatus(`Erreur: ${String(e)}`);
    } finally {
      setLoading(false);
    }
  };
  const submitPdfWithOcr = async () => {
    if (!pdfFile) return;
    setLoading(true);
    setStatus("Upload PDF...");
    try {
      const up = await fetch("/api/agentIAOCR/upload-url", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ filename: pdfFile.name, contentType: pdfFile.type || "application/pdf" }),
      });
      const upData = await up.json();
      if (!up.ok) throw new Error(upData.error || "Erreur URL signée");
      const put = await fetch(upData.url, {
        method: "PUT",
        headers: { "Content-Type": pdfFile.type || "application/pdf" },
        body: pdfFile,
      });
      if (!put.ok) throw new Error("Erreur upload S3");
      setStatus("OCR en cours...");
      const start = await fetch("/api/agentIAOCR/ocr-process", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ key: upData.key }),
      });
      const startData = await start.json();
      if (!start.ok) throw new Error(startData.error || "Erreur lancement OCR");
      let extractedText = "";
      for (let i = 0; i < 30; i++) {
        const poll = await fetch("/api/agentIAOCR/ocr-result", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ jobId: startData.jobId }),
        });
        const pollData = await poll.json();
        if (pollData.text) {
          extractedText = pollData.text;
          break;
        }
        await new Promise((r) => setTimeout(r, 3000));
      }
      if (!extractedText) throw new Error("OCR vide ou timeout");
      setStatus("Injection knowledge...");
      const data = await injectText(extractedText, `OCR PDF: ${pdfFile.name}`);
      const count = Array.isArray(data.updates) ? data.updates.length : 0;
      setStatus(`OK PDF: ${count} entrée(s) créée(s)`);
      setPdfFile(null);
    } catch (e) {setStatus(`Erreur: ${String(e)}`);
    } finally {setLoading(false)}
  };
  return (
    <ModulePageShell maxWidthClass="max-w-[1100px]">
      <ModulePageHeader
        title="Brain AI (training engine)"
        description="Injectez du texte ou des PDF. Le système classe automatiquement vers le bon JSON knowledge sur S3."
      />
      <ModuleCard className="mb-6" bodyClassName="p-5">
        <h2 className={`mb-3 font-semibold ${dash.ink}`}>Injection texte</h2>
        <input
          value={source}
          onChange={(e) => setSource(e.target.value)}
          placeholder="Source (ex: Circulaire voyage avril)"
          className={`${dash.field} mb-3`}
        />
        <textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="Collez ici le texte à injecter..."
          className={`${dash.field} min-h-[180px]`}
        />
      </ModuleCard>

      <ModuleCard className="mb-6" bodyClassName="p-5">
        <h2 className={`mb-3 font-semibold ${dash.ink}`}>Injection PDF + OCR</h2>
        <input type="file" accept="application/pdf" onChange={(e) => setPdfFile(e.target.files?.[0] ?? null)} className={`text-sm ${dash.textMid}`} />
      </ModuleCard>
      <div className="mb-4 flex items-center gap-3">
        <label className={`text-sm font-medium ${dash.ink}`}>Audience</label>
        <select
          value={audience}
          onChange={(e) => setAudience(e.target.value as "public" | "private" | "both")}
          className={`${dash.field} w-auto cursor-pointer px-2 py-1`}
        >
          <option value="both">Public + privé</option>
          <option value="public">Public</option>
          <option value="private">Privé</option>
        </select>
      </div>
      <div className="flex gap-3">
        <ModuleButton onClick={submitManual} disabled={loading}>
          Injecter texte
        </ModuleButton>
        <ModuleButton variant="secondary" onClick={submitPdfWithOcr} disabled={loading || !pdfFile}>
          OCR + Injecter PDF
        </ModuleButton>
      </div>
      {status ? <p className={`mt-4 text-sm ${dash.ink}`}>{status}</p> : null}
      {lastUpdates.length > 0 ? (
        <ModuleCard className="mt-6" bodyClassName="p-5">
          <h2 className={`mb-3 font-semibold ${dash.ink}`}>Dernier classement IA</h2>
          <div className="space-y-3">
            {lastUpdates.map((u, i) => (
              <div key={`${u.domain}-${i}`} className={`rounded-xl border bg-white/70 p-3 ${dash.borderSoft}`}>
                <p className={`text-sm font-semibold ${dash.ink}`}>{u.title}</p>
                <p className={`mt-1 text-xs ${dash.textMid}`}>
                  Domaine: <span className="font-semibold">{u.domain}</span>
                  {u.file ? ` • Fichier: ${u.file}` : ""}
                </p>
                {u.contentPreview ? (
                  <p className={`mt-2 whitespace-pre-wrap text-sm ${dash.ink}`}>{u.contentPreview}</p>
                ) : null}
              </div>
            ))}
          </div>
        </ModuleCard>
      ) : null}
    </ModulePageShell>
  );
}
