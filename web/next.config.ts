import type { NextConfig } from "next";
import { fileURLToPath } from "url";
import { dirname, resolve } from "path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const emptyModulePath = resolve(__dirname, "empty-module.cjs");

const nextConfig: NextConfig = {
  eslint: {
    ignoreDuringBuilds: true,
  },

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

  webpack: (config) => {
    config.resolve = config.resolve ?? {};
    config.resolve.alias = {
      ...(config.resolve.alias ?? {}),
      "pino-pretty": false,
      porto: emptyModulePath,
    };

    return config;
  },
};

export default nextConfig;
