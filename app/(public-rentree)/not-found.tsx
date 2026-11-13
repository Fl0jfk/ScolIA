import Link from "@/app/components/ui/AppLink";
import RentreePublicHeader from "@/app/components/RentreePublicHeader";

export default function PublicRentreeNotFound() {
  return (
    <div className="bg-white min-h-screen">
      <RentreePublicHeader />
      <main className="mx-auto max-w-lg px-4 py-20 text-center">
        <p className="text-6xl font-black text-slate-200">404</p>
        <h1 className="mt-4 text-2xl font-black text-slate-900">Page introuvable</h1>
        <p className="mt-3 text-sm text-slate-600">
          Cette page n&apos;est pas disponible pour cet établissement ou n&apos;a pas encore été activée.
        </p>
        <Link href="/" className="mt-8 inline-block rounded-xl bg-slate-900 px-5 py-2.5 text-sm font-bold text-white">
          Retour à l&apos;accueil
        </Link>
      </main>
    </div>
  );
}
