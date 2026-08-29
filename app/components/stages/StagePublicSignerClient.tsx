"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";

type SignMethod = "code_confirm" | "touch" | "paper_upload";

type SignView = {
  convention: {
    studentName: string;
    className: string;
    companyName: string;
    period: string;
    scheduleSummary: string;
    hasPdf: boolean;
  };
  signature: {
    role: string;
    roleLabel: string;
    label: string;
    status: string;
    signedAt?: string;
    signedBy?: string;
    reviewStatus?: string;
    signMethod?: string;
  };
  isExternalSigner: boolean;
  stampsPdf: boolean;
  needsDrawnSignature: boolean;
  hasStoredReferentSignature: boolean;
  pdfUrl: string | null;
  pdfDownloadUrl: string | null;
};

function SignatureCanvas({ onChange }: { onChange: (dataUrl: string | null) => void }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const drawing = useRef(false);

  const getPos = (e: React.MouseEvent | React.TouchEvent) => {
    const canvas = canvasRef.current!;
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    if ("touches" in e) {
      const t = e.touches[0];
      return { x: (t.clientX - rect.left) * scaleX, y: (t.clientY - rect.top) * scaleY };
    }
    return { x: (e.clientX - rect.left) * scaleX, y: (e.clientY - rect.top) * scaleY };
  };

  const start = (e: React.MouseEvent | React.TouchEvent) => {
    e.preventDefault();
    drawing.current = true;
    const ctx = canvasRef.current?.getContext("2d");
    if (!ctx) return;
    const { x, y } = getPos(e);
    ctx.beginPath();
    ctx.moveTo(x, y);
  };

  const move = (e: React.MouseEvent | React.TouchEvent) => {
    if (!drawing.current) return;
    e.preventDefault();
    const ctx = canvasRef.current?.getContext("2d");
    if (!ctx) return;
    const { x, y } = getPos(e);
    ctx.lineWidth = 2.5;
    ctx.lineCap = "round";
    ctx.strokeStyle = "#1a1a1a";
    ctx.lineTo(x, y);
    ctx.stroke();
    onChange(canvasRef.current?.toDataURL("image/png") ?? null);
  };

  const end = () => {
    drawing.current = false;
    onChange(canvasRef.current?.toDataURL("image/png") ?? null);
  };

  const clear = () => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    canvas.getContext("2d")?.clearRect(0, 0, canvas.width, canvas.height);
    onChange(null);
  };

  return (
    <div>
      <p className="text-xs font-semibold text-stone-600 mb-2">
        Signez avec le doigt ou la souris dans le cadre ci-dessous
      </p>
      <canvas
        ref={canvasRef}
        width={400}
        height={120}
        className="w-full touch-none rounded-lg border-2 border-dashed border-stone-300 bg-white cursor-crosshair"
        onMouseDown={start}
        onMouseMove={move}
        onMouseUp={end}
        onMouseLeave={end}
        onTouchStart={start}
        onTouchMove={move}
        onTouchEnd={end}
      />
      <button
        type="button"
        onClick={clear}
        className="mt-2 text-xs font-semibold text-stone-500 underline hover:text-stone-800"
      >
        Effacer
      </button>
    </div>
  );
}

function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ""));
    reader.onerror = () => reject(new Error("Lecture fichier impossible"));
    reader.readAsDataURL(file);
  });
}

export default function StagePublicSignerClient() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const initialToken = searchParams.get("token") || "";
  const [token, setToken] = useState(initialToken);
  const [view, setView] = useState<SignView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [signerName, setSignerName] = useState("");
  const [signaturePng, setSignaturePng] = useState<string | null>(null);
  const [paperFile, setPaperFile] = useState<File | null>(null);
  const [signMethod, setSignMethod] = useState<SignMethod>("code_confirm");
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);
  const [pendingReview, setPendingReview] = useState(false);
  const [codeEmail, setCodeEmail] = useState("");
  const [secureCode, setSecureCode] = useState("");

  const load = useCallback(async (activeToken: string) => {
    if (!activeToken) {
      setView(null);
      return;
    }
    const res = await fetch(`/api/stages/public/sign?token=${encodeURIComponent(activeToken)}`, {
      cache: "no-store",
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data?.error || "Lien invalide");
    setView(data);
    if (data.signature?.status === "signe") {
      setDone(true);
      setPendingReview(data.signature?.reviewStatus === "pending");
    }
    if (data.isExternalSigner) {
      setSignMethod("touch");
    }
  }, []);

  useEffect(() => {
    if (initialToken) {
      void load(initialToken).catch((e: unknown) =>
        setError(e instanceof Error ? e.message : "Erreur"),
      );
    }
  }, [initialToken, load]);

  async function resolveCode(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/stages/public/sign", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "resolve_code", email: codeEmail, code: secureCode }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Code invalide");
      setToken(data.token);
      router.replace(`/stages/signer?token=${encodeURIComponent(data.token)}`);
      await load(data.token);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function sign(chosenMethod?: SignMethod) {
    if (!view || !token) return;
    const method = chosenMethod ?? signMethod;

    if (method === "touch" && !signaturePng && !view.hasStoredReferentSignature && view.needsDrawnSignature) {
      setError("Dessinez votre signature dans le cadre ci-dessous.");
      return;
    }
    if (method === "touch" && view.isExternalSigner && !signaturePng) {
      setError("Dessinez votre signature dans le cadre ci-dessous.");
      return;
    }
    if (method === "paper_upload" && !paperFile) {
      setError("Déposez le PDF signé (glisser-déposer ou parcourir).");
      return;
    }

    setBusy(true);
    setError(null);
    try {
      let paperPdfBase64: string | undefined;
      if (method === "paper_upload" && paperFile) {
        paperPdfBase64 = await fileToBase64(paperFile);
      }

      const res = await fetch("/api/stages/public/sign", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          token,
          signerName,
          signMethod: method,
          signaturePngBase64: method === "touch" ? signaturePng || undefined : undefined,
          paperPdfBase64,
          paperFileName: paperFile?.name,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setDone(true);
      setPendingReview(method === "touch" || method === "paper_upload");
      await load(token);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  if (!token) {
    return (
      <main className="min-h-screen bg-[#f6f8f5] px-4 py-10">
        <div className="mx-auto max-w-md rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
          <h1 className="text-2xl font-black text-[#1F3D2B]">Signer une convention</h1>
          <p className="mt-2 text-sm text-stone-600">
            Saisissez l&apos;e-mail sur lequel vous avez reçu le code sécurisé à 6 chiffres.
          </p>
          {error && <p className="mt-4 text-sm text-rose-700">{error}</p>}
          <form onSubmit={(e) => void resolveCode(e)} className="mt-6 space-y-4 text-sm">
            <input
              className="w-full rounded-lg border px-3 py-2"
              type="email"
              placeholder="Votre e-mail"
              value={codeEmail}
              onChange={(e) => setCodeEmail(e.target.value)}
              required
            />
            <input
              className="w-full rounded-lg border px-3 py-2 font-mono tracking-widest text-center text-lg"
              inputMode="numeric"
              pattern="[0-9]{6}"
              maxLength={6}
              placeholder="Code 6 chiffres"
              value={secureCode}
              onChange={(e) => setSecureCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
              required
            />
            <button
              type="submit"
              disabled={busy}
              className="w-full rounded-lg bg-[#2F6B4A] py-3 text-sm font-bold text-white disabled:opacity-50"
            >
              {busy ? "Vérification…" : "Accéder à la convention"}
            </button>
          </form>
        </div>
      </main>
    );
  }

  if (!view && !error) {
    return <main className="min-h-screen flex items-center justify-center p-6">Chargement…</main>;
  }

  if (error && !view) {
    return (
      <main className="min-h-screen flex items-center justify-center p-6">
        <p className="text-rose-700">{error}</p>
      </main>
    );
  }

  if (!view) return null;

  const isDirection = view.signature.role === "direction";
  const isProf = view.signature.role === "professeur_referent";

  return (
    <main className="min-h-screen bg-[#f6f8f5] px-4 py-10">
      <div className="mx-auto max-w-2xl rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <h1 className="text-2xl font-black text-[#1F3D2B]">Signature convention de stage</h1>
        <p className="mt-2 text-sm text-stone-600">En tant que : {view.signature.roleLabel}</p>

        {view.stampsPdf && !view.isExternalSigner && (
          <p className="mt-3 rounded-lg border border-blue-200 bg-blue-50 px-3 py-2 text-xs text-blue-900">
            {isDirection
              ? "Votre signature enregistrée (direction) sera apposée directement sur le PDF."
              : view.hasStoredReferentSignature
                ? "Votre signature enregistrée (Mon compte → Sécurité) sera apposée sur le PDF."
                : isProf
                  ? "Enregistrez d'abord votre signature dans Mon compte → Sécurité → Ma signature, ou dessinez-la ci-dessous."
                  : "Dessinez votre signature : elle sera intégrée au PDF de la convention."}
          </p>
        )}

        <div className="mt-6 rounded-xl bg-stone-50 p-4 text-sm space-y-2">
          <p>
            <strong>Élève :</strong> {view.convention.studentName} ({view.convention.className})
          </p>
          <p>
            <strong>Entreprise :</strong> {view.convention.companyName}
          </p>
          <p>
            <strong>Période :</strong> {view.convention.period}
          </p>
          <p>
            <strong>Horaires :</strong> {view.convention.scheduleSummary}
          </p>
        </div>

        {view.pdfUrl && (
          <div className="mt-6">
            <p className="text-xs font-bold text-stone-600 mb-2">Aperçu du document</p>
            <iframe
              title="Convention PDF"
              src={view.pdfUrl}
              className="h-[420px] w-full rounded-lg border border-stone-200"
            />
          </div>
        )}

        {done ? (
          <p className="mt-6 rounded-lg bg-emerald-50 border border-emerald-200 px-4 py-3 text-sm text-emerald-800">
            {pendingReview
              ? "Votre signature a été transmise et sera validée par l'établissement sous peu."
              : `Signature enregistrée${view.signature.signedBy ? ` par ${view.signature.signedBy}` : ""}${view.stampsPdf ? " — paraphe ajouté sur le PDF." : "."}`}
          </p>
        ) : view.isExternalSigner ? (
          <div className="mt-6 space-y-5">
            <input
              className="w-full rounded-lg border border-stone-300 px-3 py-2 text-sm"
              placeholder="Votre nom (ex. M. Dupont)"
              value={signerName}
              onChange={(e) => setSignerName(e.target.value)}
            />

            <div className="flex flex-wrap gap-2">
              {(
                [
                  ["code_confirm", "Code e-mail"],
                  ["touch", "Signer au doigt"],
                  ["paper_upload", "Document papier"],
                ] as const
              ).map(([id, label]) => (
                <button
                  key={id}
                  type="button"
                  onClick={() => setSignMethod(id)}
                  className={`rounded-lg px-3 py-2 text-xs font-bold border ${
                    signMethod === id
                      ? "border-[#2F6B4A] bg-[#2F6B4A] text-white"
                      : "border-stone-300 text-stone-700"
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>

            {signMethod === "code_confirm" && (
              <div className="rounded-lg border border-stone-200 bg-stone-50 p-4 text-sm text-stone-700">
                <p>
                  Confirmez avoir lu la convention et autorisez la signature électronique simple. Le
                  code reçu par e-mail atteste de votre identité.
                </p>
              </div>
            )}

            {signMethod === "touch" && <SignatureCanvas onChange={setSignaturePng} />}

            {signMethod === "paper_upload" && (
              <div className="space-y-3">
                {view.pdfDownloadUrl && (
                  <a
                    href={view.pdfDownloadUrl}
                    className="inline-flex rounded-lg bg-stone-800 px-4 py-2 text-sm font-bold text-white"
                  >
                    Télécharger la convention (PDF)
                  </a>
                )}
                <p className="text-xs text-stone-600">
                  Imprimez, signez en papier, puis déposez le scan ou la photo PDF ci-dessous.
                </p>
                <label className="flex min-h-[120px] cursor-pointer flex-col items-center justify-center rounded-xl border-2 border-dashed border-stone-300 bg-stone-50 px-4 py-6 text-center text-sm text-stone-600 hover:border-[#2F6B4A]">
                  <input
                    type="file"
                    accept="application/pdf,image/*"
                    className="hidden"
                    onChange={(e) => setPaperFile(e.target.files?.[0] ?? null)}
                  />
                  {paperFile ? paperFile.name : "Glisser-déposer ou cliquer pour choisir un fichier"}
                </label>
              </div>
            )}

            {error && <p className="text-sm text-rose-700">{error}</p>}
            <button
              type="button"
              disabled={busy}
              onClick={() => void sign()}
              className="w-full rounded-lg bg-[#2F6B4A] py-3 text-sm font-bold text-white disabled:opacity-50"
            >
              {busy ? "Envoi…" : "Valider ma signature"}
            </button>
          </div>
        ) : (
          <div className="mt-6 space-y-4">
            <input
              className="w-full rounded-lg border border-stone-300 px-3 py-2 text-sm"
              placeholder="Votre nom (ex. M. Dupont)"
              value={signerName}
              onChange={(e) => setSignerName(e.target.value)}
            />

            {view.needsDrawnSignature && <SignatureCanvas onChange={setSignaturePng} />}

            {error && <p className="text-sm text-rose-700">{error}</p>}
            <button
              type="button"
              disabled={busy}
              onClick={() => void sign("code_confirm")}
              className="w-full rounded-lg bg-[#2F6B4A] py-3 text-sm font-bold text-white disabled:opacity-50"
            >
              {view.stampsPdf ? "Signer et apposer sur le PDF" : "Signer la convention"}
            </button>
          </div>
        )}
      </div>
    </main>
  );
}
