import JSZip from "jszip";
import type { OfficeKind } from "@/app/lib/office-types";
import { OFFICE_KIND_META } from "@/app/lib/office-types";

function buildManifest(mime: string): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.3">
 <manifest:file-entry manifest:full-path="/" manifest:media-type="${mime}" manifest:version="1.3"/>
 <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
 <manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/>
 <manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>
</manifest:manifest>
`;
}

const STYLES_XML = `<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
 xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"
 office:version="1.3">
 <office:styles/>
 <office:automatic-styles/>
 <office:master-styles/>
</office:document-styles>
`;

function metaXml(): string {
  const now = new Date().toISOString();
  return `<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0"
 xmlns:dc="http://purl.org/dc/elements/1.1/"
 office:version="1.3">
 <office:meta>
  <meta:generator>ScolIA</meta:generator>
  <meta:creation-date>${now}</meta:creation-date>
  <dc:date>${now}</dc:date>
 </office:meta>
</office:document-meta>
`;
}

function contentWriter(): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
 office:version="1.3">
 <office:scripts/>
 <office:font-face-decls/>
 <office:automatic-styles/>
 <office:body>
  <office:text>
   <text:p text:style-name="Standard"/>
  </office:text>
 </office:body>
</office:document-content>
`;
}

function contentCalc(): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 office:version="1.3">
 <office:scripts/>
 <office:font-face-decls/>
 <office:automatic-styles/>
 <office:body>
  <office:spreadsheet>
   <table:table table:name="Feuille1">
    <table:table-column/>
    <table:table-row>
     <table:table-cell office:value-type="string"><text:p/></table:table-cell>
    </table:table-row>
   </table:table>
  </office:spreadsheet>
 </office:body>
</office:document-content>
`;
}

function contentImpress(): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
 xmlns:presentation="urn:oasis:names:tc:opendocument:xmlns:presentation:1.0"
 xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 office:version="1.3">
 <office:scripts/>
 <office:font-face-decls/>
 <office:automatic-styles>
  <style:style style:name="dp1" style:family="drawing-page"/>
 </office:automatic-styles>
 <office:body>
  <office:presentation>
   <draw:page draw:name="page1" draw:style-name="dp1" draw:master-page-name="Default">
    <draw:frame svg:width="16cm" svg:height="2cm" svg:x="2cm" svg:y="4cm">
     <draw:text-box><text:p/></draw:text-box>
    </draw:frame>
   </draw:page>
  </office:presentation>
 </office:body>
</office:document-content>
`;
}

function contentFor(kind: OfficeKind): string {
  if (kind === "calc") return contentCalc();
  if (kind === "impress") return contentImpress();
  return contentWriter();
}

/** Génère un fichier ODF vide valide (zip). */
export async function buildBlankOfficeBuffer(kind: OfficeKind): Promise<Buffer> {
  const mime = OFFICE_KIND_META[kind].mime;
  const zip = new JSZip();
  zip.file("mimetype", mime, { compression: "STORE" });
  zip.file("META-INF/manifest.xml", buildManifest(mime));
  zip.file("content.xml", contentFor(kind));
  zip.file("styles.xml", STYLES_XML);
  zip.file("meta.xml", metaXml());
  const out = await zip.generateAsync({
    type: "nodebuffer",
    compression: "DEFLATE",
    compressionOptions: { level: 6 },
  });
  return Buffer.from(out);
}
