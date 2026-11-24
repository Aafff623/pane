import React from "react";

import { cn } from "@/lib/utils";

type BackgroundProps = {
  children: React.ReactNode;
  variant?: "top" | "bottom";
  className?: string;
};

export const Background = ({ children, variant = "top", className }: BackgroundProps) => {
  return (
    <div className={cn(variant === "bottom" && "rounded-3xl bg-accent/50", className)}>
      {children}
    </div>
  );
};
