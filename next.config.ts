import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: "standalone",
  // Dev : Playwright / curl via 127.0.0.1 (HMR + assets)
  allowedDevOrigins: ["127.0.0.1", "localhost"],
  serverExternalPackages: ["@napi-rs/canvas", "pdfjs-dist"],
  typescript: { ignoreBuildErrors: false },
  /**
   * Imports Siècle (XML Établissements / Élèves / Responsables jusqu'à ~100 Mo)
   * passent par proxy.ts : le body est bufferisé — défaut Next = 10 Mo → formData tronqué → 400.
   */
  experimental: {
    proxyClientMaxBodySize: "110mb",
    serverActions: {
      bodySizeLimit: "110mb",
    },
  },
  images: {
    formats : ['image/webp'],
    remotePatterns: [
        // Scaleway Object Storage — fr-par (virtual-hosted)
        {
            protocol: 'https',
            hostname: '*.s3.fr-par.scw.cloud',
            pathname: '/**',
        },
        // Scaleway Object Storage — fr-par (path-style)
        {
            protocol: 'https',
            hostname: 's3.fr-par.scw.cloud',
            pathname: '/**',
        },
        {
            protocol: 'https',
            hostname: 'flagcdn.com',
            pathname: '/**',
        },
        {
            protocol: 'https',
            hostname: 'images.unsplash.com',
            pathname: '/**',
        }
    ]
  },
  async headers() {
    const securityHeaders = [
      {
        key: "Referrer-Policy",
        value: "strict-origin-when-cross-origin",
      },
      {
        key: "Permissions-Policy",
        value: "camera=(self), microphone=(self), geolocation=()",
      },
      ...(process.env.NODE_ENV === "production"
        ? [
            {
              key: "Strict-Transport-Security",
              value: "max-age=63072000; includeSubDomains; preload",
            },
          ]
        : []),
    ];
    return [
      {
        // CSP + COOP + X-Frame-Options : uniquement pages HTML via proxy.ts.
        // Sur les fetch RSC (?_rsc=) Safari bloque sinon (« access control checks »).
        source: "/((?!api/)(?!documents/rentree/).*)",
        headers: securityHeaders,
      },
    ];
  },
  async rewrites() {
    const raw = process.env.COLLABORA_URL?.trim() || process.env.NEXT_PUBLIC_COLLABORA_URL?.trim();
    if (!raw || process.env.OFFICE_SAME_ORIGIN === "1") return [];
    let origin: string;
    try {
      origin = new URL(raw).origin;
    } catch {
      return [];
    }
    return [
      { source: "/browser/:path*", destination: `${origin}/browser/:path*` },
      { source: "/cool/:path*", destination: `${origin}/cool/:path*` },
      { source: "/hosting/:path*", destination: `${origin}/hosting/:path*` },
    ];
  },
};

export default nextConfig;