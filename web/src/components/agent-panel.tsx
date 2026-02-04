"use client";

import { useState } from "react";
import { Send } from "lucide-react";
import { AomiFrame } from "@/components/aomi-frame";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";

interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: Date;
}

interface AgentPanelProps {
  title: string;
  role: "maker" | "taker";
  agentId?: string;
}

// Placeholder for AomiFrame integration
// Installed via shadcn: npx shadcn add https://aomi.dev/r/aomi-frame.json
export function AgentPanel({ title, role, agentId }: AgentPanelProps) {
  const [messages, setMessages] = useState<Message[]>([
    {
      id: "1",
      role: "assistant",
      content:
        role === "maker"
          ? "Hello! I'm your Maker agent. I can help you create RFQ quotes in plain English. Try saying something like: \"I want to buy 10 dETH at most 2000 USDD, expires in 5 minutes.\""
          : "Hello! I'm your Taker agent. I can help you find and fill quotes. Tell me what you're looking for, or I can show you active quotes in the market.",
      timestamp: new Date(),
    },
  ]);
  const [input, setInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  const handleSend = async () => {
    if (!input.trim() || isLoading) return;

    const userMessage: Message = {
      id: Date.now().toString(),
      role: "user",
      content: input,
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setInput("");
    setIsLoading(true);

    // Simulate agent response (replace with actual AomiFrame integration)
    setTimeout(() => {
      const assistantMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: "assistant",
        content:
          role === "maker"
            ? `I understand you want to create a quote. Let me compile that into guardrails...\n\n[Demo mode: AomiFrame integration pending. In production, this would call the /quotes API with your natural language input.]`
            : `Looking for quotes matching your criteria...\n\n[Demo mode: AomiFrame integration pending. In production, this would search quotes and help you fill them.]`,
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, assistantMessage]);
      setIsLoading(false);
    }, 1000);
  };

  const roleColor = role === "maker" ? "bg-green-500" : "bg-blue-500";

  return (
    <div className="flex flex-col h-full border rounded-lg bg-card overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b bg-muted/50">
        <div className="flex items-center gap-2">
          <div className={`w-2 h-2 rounded-full ${roleColor}`} />
          <h3 className="font-medium">{title}</h3>
          <Badge variant="outline" className="text-xs">
            {role}
          </Badge>
        </div>
        {agentId && (
          <span className="text-xs text-muted-foreground font-mono">
            Agent: {agentId.slice(0, 8)}...
          </span>
        )}
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {messages.map((message) => (
          <div
            key={message.id}
            className={`flex ${message.role === "user" ? "justify-end" : "justify-start"}`}
          >
            <div
              className={`max-w-[80%] rounded-lg px-4 py-2 ${
                message.role === "user"
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted"
              }`}
            >
              <p className="text-sm whitespace-pre-wrap">{message.content}</p>
              <span className="text-[10px] opacity-70">
                {message.timestamp.toLocaleTimeString()}
              </span>
            </div>
          </div>
        ))}
        {isLoading && (
          <div className="flex justify-start">
            <div className="bg-muted rounded-lg px-4 py-2">
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-primary animate-bounce" />
                <div className="w-2 h-2 rounded-full bg-primary animate-bounce delay-100" />
                <div className="w-2 h-2 rounded-full bg-primary animate-bounce delay-200" />
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Input */}
      <div className="p-4 border-t bg-muted/30">
        <form
          onSubmit={(e) => {
            e.preventDefault();
            handleSend();
          }}
          className="flex items-center gap-2"
        >
          <Input
            placeholder={
              role === "maker"
                ? "Describe your quote in English..."
                : "What are you looking to buy/sell?"
            }
            value={input}
            onChange={(e) => setInput(e.target.value)}
            disabled={isLoading}
            className="flex-1"
          />
          <Button type="submit" size="icon" disabled={isLoading || !input.trim()}>
            <Send className="h-4 w-4" />
          </Button>
        </form>
        <p className="text-[10px] text-muted-foreground mt-2">
          Powered by Aomi Runtime. Messages are processed by AI agents.
        </p>
      </div>
    </div>
  );
}

// AomiFrame integration installed via shadcn

export function AomiFrameWrapper({
  role,
  height = "100%",
  width = "100%",
}: {
  role: "maker" | "taker";
  height?: string;
  width?: string;
}) {
  return (
    <AomiFrame.Root key={role} height={height} width={width} walletPosition="footer">
      <AomiFrame.Header />
      <AomiFrame.Composer withControl />
    </AomiFrame.Root>
  );
}
