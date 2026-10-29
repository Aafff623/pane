"use client";

import { DownloadSimple } from "@phosphor-icons/react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { usePlatformTarget } from "@/components/download-cta";

type Props = {
  labels: { windows: string; macos: string };
  className?: string;
  size?: "sm" | "default" | "lg";
  variant?: "default" | "outline" | "ink";
  showIcon?: boolean;
};

export function DownloadButton({ labels, className, size = "default", variant = "default", showIcon = true }: Props) {
  const target = usePlatformTarget(labels);
  const external = target.href.startsWith("http");
  return (
    <Button asChild size={size} variant={variant} className={cn(className)}>
      <a href={target.href} {...(external ? { target: "_blank", rel: "noopener noreferrer" } : {})}>
        {showIcon ? <DownloadSimple size={16} weight="bold" aria-hidden="true" /> : null}
        {target.label}
      </a>
    </Button>
  );
}
