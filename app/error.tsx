"use client";

type Props = {
  error: Error & { digest?: string };
  reset: () => void;
};

export default function GlobalError({ error, reset }: Props) {
  return (
    <div className="flex min-h-[50vh] items-center justify-center px-4">
      <div className="w-full max-w-md rounded-2xl border border-black/8 bg-white p-8 text-center shadow-sm">
        <h1 className="text-lg font-semibold text-[var(--dash-ink,#1d1d1f)]">
          Un problème est survenu
        </h1>
        <p className="mt-2 text-sm leading-relaxed text-neutral-600">
          L&apos;affichage a été interrompu. Réessayez ou rechargez la page.
        </p>
        {process.env.NODE_ENV === "development" && error.message ? (
          <p className="mt-3 break-words text-left text-xs text-red-700">{error.message}</p>
        ) : null}
        <button
          type="button"
          onClick={() => reset()}
          className="mt-6 rounded-xl bg-[var(--dash-ink,#1d1d1f)] px-5 py-2.5 text-sm font-semibold text-white hover:opacity-90"
        >
          Réessayer
        </button>
      </div>
    </div>
  );
}
