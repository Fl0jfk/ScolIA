import { permanentRedirect } from "next/navigation";

type Props = { searchParams?: Promise<Record<string, string | string[] | undefined>> };

export default async function FamilleLegacysanctionsRedirect({ searchParams }: Props) {
  const sp = (await searchParams) || {};
  const qs = new URLSearchParams();
  for (const [k, v] of Object.entries(sp)) {
    if (typeof v === "string") qs.set(k, v);
    else if (Array.isArray(v) && v[0]) qs.set(k, v[0]);
  }
  const q = qs.toString();
  permanentRedirect(q ? `/quotidien/sanctions?${q}` : `/quotidien/sanctions`);
}
