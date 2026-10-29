"use client";

import { useState } from "react";

import { Check, Copy } from "lucide-react";

import { Button } from "@/components/ui/button";

export function CopyPrompt({ text }: { text: string }) {
  const [status, setStatus] = useState<"idle" | "copied" | "error">("idle");
  const [pending, setPending] = useState(false);

  async function copy() {
    setPending(true);
    try {
      await navigator.clipboard.writeText(text);
      setStatus("copied");
    } catch {
      setStatus("error");
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="mt-6 flex flex-wrap items-center gap-4">
      <Button
        onClick={copy}
        disabled={pending}
        aria-describedby="copy-prompt-status"
      >
        {status === "copied" ? (
          <Check aria-hidden="true" />
        ) : (
          <Copy aria-hidden="true" />
        )}
        {pending
          ? "Copying…"
          : status === "copied"
            ? "Copied setup prompt"
            : "Copy setup prompt"}
      </Button>
      <p
        id="copy-prompt-status"
        role="status"
        className="text-muted-foreground text-base"
      >
        {status === "error"
          ? "Could not copy. Select the prompt above and copy it manually."
          : status === "copied"
            ? "Paste it into your AI app to begin."
            : "No token or personal data in this prompt."}
      </p>
    </div>
  );
}
