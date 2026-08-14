"use client";

import ModuleButton from "@/app/components/module-chrome/ModuleButton";

export default function OcrOneDriveConnectBar({
  dropsAvailable,
  accountName,
  clerkUnmapped,
  oneDriveProfile,
  checkingOneDrive,
  showReconnect,
  onLogin,
  onReconnect,
}: {
  dropsAvailable: boolean;
  accountName?: string | null;
  clerkUnmapped?: { lastName?: string | null; email?: string | null } | null;
  oneDriveProfile?: { label: string; basePath: string } | null;
  checkingOneDrive: boolean;
  showReconnect: boolean;
  onLogin: () => void;
  onReconnect: () => void;
}) {
  return (
    <>
      {clerkUnmapped ? (
        <div className="mb-6 p-4 bg-amber-50 border-l-4 border-amber-500 text-amber-900 rounded-r-xl">
          <p className="font-bold">Profil OneDrive non reconnu</p>
          <p className="text-sm">
            Votre compte Clerk ({clerkUnmapped.lastName || "nom absent"} —{" "}
            {clerkUnmapped.email || "e-mail absent"}) n&apos;est pas encore associé au dossier collège /
            lycée / école. Contactez l&apos;administrateur pour l&apos;ajouter.
          </p>
        </div>
      ) : null}

      {oneDriveProfile ? (
        <div className="mb-6 p-4 bg-slate-50 border border-slate-200 text-slate-700 rounded-xl text-sm">
          Dossier OneDrive configuré : <strong>{oneDriveProfile.label}</strong> —{" "}
          <span className="font-mono text-xs">{oneDriveProfile.basePath}</span>
        </div>
      ) : null}

      <div
        data-tour="onedrive-connect"
        className={`mb-8 rounded-3xl border p-5 md:p-6 ${
          dropsAvailable ? "border-green-200 bg-green-50/60" : "border-amber-200 bg-amber-50/80"
        }`}
      >
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
          <div>
            <p className="text-[10px] font-black uppercase tracking-wider text-slate-500 mb-1">
              Étape 1 — Connexion
            </p>
            <h2 className="text-lg font-bold text-slate-900">
              {dropsAvailable ? "OneDrive est connecté" : "Connectez-vous à OneDrive"}
            </h2>
            <p className="text-sm text-slate-600 mt-1 max-w-xl">
              {dropsAvailable
                ? accountName
                  ? `Compte : ${accountName}. Vous pouvez déposer vos PDF ci-dessous.`
                  : "Vous pouvez déposer vos PDF ci-dessous."
                : "La connexion Microsoft est obligatoire avant tout dépôt. Sans OneDrive, l'analyse et le rangement ne sont pas possibles."}
            </p>
          </div>
          {!dropsAvailable ? (
            <div className="flex flex-wrap gap-2 shrink-0">
              <ModuleButton onClick={onLogin} className="px-6 py-3 shadow-lg">
                Se connecter à OneDrive
              </ModuleButton>
              {showReconnect ? (
                <ModuleButton variant="secondary" onClick={onReconnect} className="px-6 py-3">
                  Reconnecter (consentement)
                </ModuleButton>
              ) : null}
            </div>
          ) : null}
        </div>
      </div>

      {checkingOneDrive ? (
        <div className="mb-6 p-4 bg-blue-50 border border-blue-200 rounded-2xl text-blue-800 text-sm font-medium flex items-center gap-3">
          <div className="animate-spin rounded-full h-5 w-5 border-2 border-blue-600 border-t-transparent" />
          Vérification de la connexion OneDrive…
        </div>
      ) : null}
    </>
  );
}
