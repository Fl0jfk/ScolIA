import { notFound } from "next/navigation";
import { loadVerifySnapshot } from "@/app/lib/certificates-storage";
import { shortContentHash } from "@/app/lib/certificates-verify";
import { CERTIFICATE_SECTEUR_LABELS } from "@/app/lib/certificates-types";

type VerifyPageParams = { params: Promise<{ token: string }> };

export default async function CertificateVerifyPage({ params }: VerifyPageParams) {
  const { token } = await params;
  if (!token?.trim()) notFound();
  const data = await loadVerifySnapshot(token);
  if (!data) notFound();
  const shortHash = shortContentHash(data.contentHash);

  return (
    <div className="min-h-screen bg-gradient-to-b from-indigo-50 to-white py-10 px-4">
      <div className="max-w-2xl mx-auto rounded-3xl border border-slate-200 bg-white shadow-xl overflow-hidden">
        <div className="bg-emerald-600 px-6 py-4 text-white">
          <p className="text-xs font-black uppercase tracking-wider opacity-90">Document authentique</p>
          <h1 className="text-2xl font-black mt-1">{data.programTitle}</h1>
          {data.tenantName && <p className="text-sm opacity-90 mt-1">{data.tenantName}</p>}
        </div>
        <div className="p-6 space-y-6">
          <div>
            <p className="text-xs font-bold text-slate-400 uppercase">Décerné à</p>
            <p className="text-xl font-black text-slate-900">
              {data.student.prenom} {data.student.nom}
            </p>
            <p className="text-sm text-slate-600">
              {data.student.classe} · {CERTIFICATE_SECTEUR_LABELS[data.student.secteur]} · Année {data.schoolYear}
            </p>
          </div>

          <div>
            <p className="text-xs font-bold text-slate-400 uppercase mb-2">Parcours et réalisations</p>
            <ol className="space-y-3">
              {data.lines.map((line, i) => (
                <li key={line.id} className="text-sm text-slate-800">
                  <p className="font-black text-slate-900">
                    <span className="text-slate-400">{i + 1}. </span>
                    {line.title}
                    {line.period && <span className="font-normal text-slate-500"> — {line.period}</span>}
                  </p>
                  <p className="mt-1 pl-5">{line.description}</p>
                </li>
              ))}
            </ol>
          </div>

          <div>
            <p className="text-xs font-bold text-slate-400 uppercase mb-2">Signatures enseignants</p>
            <ul className="text-sm space-y-1">
              {data.designatedSignatories
                .filter((s) => s.status === "signed")
                .map((s) => (
                  <li key={s.externalUserId}>
                    {s.name}
                    {s.signedAt && (
                      <span className="text-slate-400">
                        {" "}
                        — {new Date(s.signedAt).toLocaleDateString("fr-FR")}
                      </span>
                    )}
                  </li>
                ))}
            </ul>
          </div>

          {data.directionSignature && (
            <div>
              <p className="text-xs font-bold text-slate-400 uppercase mb-1">Direction</p>
              <p className="text-sm">
                {data.directionSignature.signedByName} —{" "}
                {CERTIFICATE_SECTEUR_LABELS[data.directionSignature.level]} —{" "}
                {new Date(data.directionSignature.signedAt).toLocaleDateString("fr-FR")}
              </p>
            </div>
          )}

          <div className="border-t border-slate-100 pt-4 text-xs text-slate-500 space-y-1">
            <p>Émis le {new Date(data.issuedAt).toLocaleDateString("fr-FR")}</p>
            <p>Réf. {shortHash}</p>
          </div>
        </div>
      </div>
    </div>
  );
}
