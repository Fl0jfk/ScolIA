import type { NextConfig } from "next";
import { contentSecurityPolicyHeaderValue, contentSecurityPolicyReportOnlyHeaderValue, crossOriginOpenerPolicyHeaderValue } from "./app/lib/content-security-policy";

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
        key: "Content-Security-Policy",
        value: contentSecurityPolicyHeaderValue(),
      },
      {
        key: "Content-Security-Policy-Report-Only",
        value: contentSecurityPolicyReportOnlyHeaderValue(),
      },
      {
        key: "Cross-Origin-Opener-Policy",
        value: crossOriginOpenerPolicyHeaderValue(),
      },
    ];
    return [
      {
        // Pas de CSP sur les flux binaires : le lecteur PDF intégré de Chrome plante sinon.
        source: "/((?!api/rentree/file)(?!api/fournitures/file)(?!documents/rentree/).*)",
        headers: securityHeaders,
      },
    ];
  },
};

export default nextConfig;