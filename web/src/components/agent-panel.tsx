"use client";

import { AomiFrame } from "@aomi-labs/widget-lib";

const BACKEND_URL = process.env.NEXT_PUBLIC_BACKEND_URL || "http://localhost:8080";

interface AomiFrameWrapperProps {
  role: "maker" | "taker";
  height?: string;
  width?: string;
}

export function AomiFrameWrapper({
  role,
  height = "100%",
  width = "100%",
}: AomiFrameWrapperProps) {
  const roleColor = role === "maker" ? "bg-green-500" : "bg-blue-500";
  
  return (
    <div style={{ height, width }} className="border rounded-lg overflow-hidden">
      <AomiFrame.Root 
        backendUrl={BACKEND_URL}
        height="100%"
        width="100%"
      >
        <AomiFrame.Header 
          withControl 
          controlBarProps={{ 
            hideWallet: true,
            hideApiKey: role === "taker"  // Only show API key on maker panel
          }} 
        />
        <AomiFrame.Composer />
      </AomiFrame.Root>
    </div>
  );
}
