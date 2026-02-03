"use client";

import { useState, useRef, useEffect } from "react";
import { Send, Key } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  AomiRuntimeProvider,
  useAomiRuntime,
  useCurrentThreadMessages,
} from "@aomi-labs/react";

interface AomiChatProps {
  role: "maker" | "taker";
  height?: string;
}

function ChatInterface({ role }: { role: "maker" | "taker" }) {
  const { sendMessage, isLoading, currentThreadId } = useAomiRuntime();
  const messages = useCurrentThreadMessages();
  const [input, setInput] = useState("");
  const [apiKey, setApiKey] = useState(() => {
    if (typeof window !== "undefined") {
      return localStorage.getItem("aomi-api-key") || "";
    }
    return "";
  });
  const [showKeyInput, setShowKeyInput] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  useEffect(() => {
    if (apiKey && typeof window !== "undefined") {
      localStorage.setItem("aomi-api-key", apiKey);
    }
  }, [apiKey]);

  const handleSend = async () => {
    if (!input.trim() || isLoading) return;
    if (!apiKey) {
      setShowKeyInput(true);
      return;
    }
    await sendMessage(input.trim());
    setInput("");
  };

  const roleColor = role === "maker" ? "bg-green-500" : "bg-blue-500";
  const welcomeMessage = role === "maker"
    ? "Hello! I'm your Maker agent. Describe your quote in plain English, like: \"Sell 10 ETH for at least 2000 USDC, expires in 5 minutes.\""
    : "Hello! I'm your Taker agent. I can help you find and fill quotes. What are you looking for?";

  return (
    <div className="flex flex-col h-full border rounded-lg bg-card overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b bg-muted/50">
        <div className="flex items-center gap-2">
          <div className={`w-2 h-2 rounded-full ${roleColor}`} />
          <span className="font-medium text-sm">
            {role === "maker" ? "Maker Agent" : "Taker Agent"}
          </span>
          <Badge variant="outline" className="text-xs">{role}</Badge>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setShowKeyInput(!showKeyInput)}
          className="h-7 px-2"
        >
          <Key className="h-3 w-3 mr-1" />
          {apiKey ? "Key Set" : "Set Key"}
        </Button>
      </div>

      {/* API Key Input */}
      {showKeyInput && (
        <div className="p-2 border-b bg-muted/30">
          <Input
            type="password"
            placeholder="Enter API Key"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            className="h-8 text-sm"
          />
        </div>
      )}

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-3 space-y-3">
        {/* Welcome message */}
        {messages.length === 0 && (
          <div className="bg-muted rounded-lg px-3 py-2">
            <p className="text-sm">{welcomeMessage}</p>
          </div>
        )}
        
        {messages.map((msg, i) => (
          <div
            key={i}
            className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}
          >
            <div
              className={`max-w-[85%] rounded-lg px-3 py-2 ${
                msg.role === "user"
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted"
              }`}
            >
              <p className="text-sm whitespace-pre-wrap">
                {typeof msg.content === "string" ? msg.content : JSON.stringify(msg.content)}
              </p>
            </div>
          </div>
        ))}
        
        {isLoading && (
          <div className="flex justify-start">
            <div className="bg-muted rounded-lg px-3 py-2">
              <div className="flex items-center gap-1">
                <div className="w-1.5 h-1.5 rounded-full bg-primary animate-bounce" />
                <div className="w-1.5 h-1.5 rounded-full bg-primary animate-bounce [animation-delay:0.1s]" />
                <div className="w-1.5 h-1.5 rounded-full bg-primary animate-bounce [animation-delay:0.2s]" />
              </div>
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="p-3 border-t">
        <form
          onSubmit={(e) => {
            e.preventDefault();
            handleSend();
          }}
          className="flex gap-2"
        >
          <Input
            placeholder={role === "maker" ? "Describe your quote..." : "What do you want to trade?"}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            disabled={isLoading}
            className="flex-1 h-9"
          />
          <Button type="submit" size="sm" disabled={isLoading || !input.trim()}>
            <Send className="h-4 w-4" />
          </Button>
        </form>
      </div>
    </div>
  );
}

export function AomiChat({ role, height = "100%" }: AomiChatProps) {
  const backendUrl = process.env.NEXT_PUBLIC_BACKEND_URL || "http://localhost:8080";

  return (
    <div style={{ height }} className="w-full">
      <AomiRuntimeProvider backendUrl={backendUrl}>
        <ChatInterface role={role} />
      </AomiRuntimeProvider>
    </div>
  );
}
