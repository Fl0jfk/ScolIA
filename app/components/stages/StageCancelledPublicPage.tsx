/** Page publique affichée quand un lien de stage pointe vers une demande annulée. */
export default function StageCancelledPublicPage({
  title = "Demande de stage annulée",
  message = "Cette demande de stage a été annulée. Les liens de signature ne sont plus valides.",
}: {
  title?: string;
  message?: string;
}) {
  return (
    <main className="flex min-h-screen items-center justify-center bg-[#f6f8f5] px-4 py-10">
      <div className="w-full max-w-lg rounded-2xl border border-amber-200 bg-white p-8 text-center shadow-sm">
        <p className="text-[11px] font-bold uppercase tracking-[0.16em] text-amber-800/70">
          Stages · La Providence
        </p>
        <h1 className="mt-3 text-2xl font-black text-[#1F3D2B]">{title}</h1>
        <p className="mt-4 text-sm leading-relaxed text-stone-600">{message}</p>
        <p className="mt-6 text-xs text-stone-500">
          Si vous pensez qu&apos;il s&apos;agit d&apos;une erreur, contactez le secrétariat de
          l&apos;établissement.
        </p>
      </div>
    </main>
  );
}
