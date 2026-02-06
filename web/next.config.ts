import type { NextConfig } from "next";
import { fileURLToPath } from "url";

const emptyModulePath = fileURLToPath(new URL("./empty-module.mjs", import.meta.url));

const nextConfig: NextConfig = {
  typescript: {
    ignoreBuildErrors: true,
  },

  images: {
    unoptimized: true,
  },

  serverExternalPackages: ["porto"],

  transpilePackages: [
    "@reown/appkit",
    "@reown/appkit-adapter-wagmi",
  ],

  turbopack: {
    resolveAlias: {
      "pino-pretty": { browser: "./empty-module.mjs" },
      porto: emptyModulePath,
    },
  },
};

export default nextConfig;
