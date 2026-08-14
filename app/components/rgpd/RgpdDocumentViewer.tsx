"use client";

type RgpdDocumentSectionView = {
  heading: string;
  paragraphs?: string[];
  bullets?: string[];
};

type Props = {
  title: string;
  sections: RgpdDocumentSectionView[];
  disclaimer?: string;
  establishmentHint?: string;
};

export default function RgpdDocumentViewer({
  title,
  sections,
  disclaimer,
  establishmentHint,
}: Props) {
  return (
    <article className="rgpd-document-viewer rounded-xl border border-slate-200 bg-white shadow-sm overflow-hidden">
      <header className="border-b border-slate-100 bg-gradient-to-r from-indigo-50 to-white px-6 py-5">
        <p className="text-[10px] font-bold uppercase tracking-wider text-indigo-600">
          Document RGPD
        </p>
        <h2 className="text-lg font-black text-slate-900 mt-1">{title}</h2>
        {establishmentHint && (
          <p className="text-xs text-slate-500 mt-1">{establishmentHint}</p>
        )}
      </header>
      <div className="px-6 py-5 space-y-6 max-h-[min(70vh,720px)] overflow-y-auto text-sm text-slate-800 leading-relaxed">
        {sections.map((section, idx) => (
          <section key={`${section.heading}-${idx}`}>
            <h3 className="text-sm font-bold text-slate-900 mb-2">{section.heading}</h3>
            {section.paragraphs?.map((p, i) => (
              <p key={i} className="mb-2 whitespace-pre-wrap">
                {p}
              </p>
            ))}
            {section.bullets && section.bullets.length > 0 && (
              <ul className="list-disc pl-5 space-y-1.5">
                {section.bullets.map((b, i) => (
                  <li key={i} className="whitespace-pre-wrap">
                    {b}
                  </li>
                ))}
              </ul>
            )}
          </section>
        ))}
      </div>
      {disclaimer && (
        <footer className="border-t border-amber-100 bg-amber-50 px-6 py-3 text-[11px] text-amber-950">
          {disclaimer}
        </footer>
      )}
    </article>
  );
}
