import type { NextConfig } from "next";
import { crossOriginOpenerPolicyHeaderValue } from "./app/lib/content-security-policy";

const nextConfig: NextConfig = {
  output: "standalone",
  serverExternalPackages: ["@napi-rs/canvas", "pdfjs-dist"],
  typescript: { ignoreBuildErrors: false },
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
        key: "Cross-Origin-Opener-Policy",
        value: crossOriginOpenerPolicyHeaderValue(),
      },
    ];
    return [
      {
        // CSP (nonce) posée dans proxy.ts — pas ici, sinon AND sans nonce casse strict-dynamic.
        // Pas de COOP non plus sur les flux PDF binaires (lecteur Chrome).
        source: "/((?!api/rentree/file)(?!api/fournitures/file)(?!documents/rentree/).*)",
        headers: securityHeaders,
      },
    ];
  },
};

export default nextConfig;