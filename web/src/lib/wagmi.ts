import { http, createConfig, createStorage, cookieStorage } from "wagmi";
import { mainnet, sepolia, arbitrum, arbitrumSepolia } from "wagmi/chains";

// Delta testnet chain config (customize as needed)
export const deltaTestnet = {
  id: 9999,
  name: "Delta Testnet",
  nativeCurrency: {
    decimals: 18,
    name: "Delta ETH",
    symbol: "dETH",
  },
  rpcUrls: {
    default: { http: ["https://rpc.testnet.delta.network"] },
  },
  blockExplorers: {
    default: { name: "Delta Explorer", url: "https://explorer.testnet.delta.network" },
  },
  testnet: true,
} as const;

export const config = createConfig({
  chains: [deltaTestnet, mainnet, sepolia, arbitrum, arbitrumSepolia],
  transports: {
    [deltaTestnet.id]: http(),
    [mainnet.id]: http(),
    [sepolia.id]: http(),
    [arbitrum.id]: http(),
    [arbitrumSepolia.id]: http(),
  },
  storage: createStorage({
    storage: cookieStorage,
  }),
  ssr: true,
});

declare module "wagmi" {
  interface Register {
    config: typeof config;
  }
}
