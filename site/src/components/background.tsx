import React from "react";

import { cn } from "@/lib/utils";

type BackgroundProps = {
  children: React.ReactNode;
  variant?: "top" | "bottom";
  className?: string;
};

export const Background = ({ children, className }: BackgroundProps) => {
  return <div className={cn("relative", className)}>{children}</div>;
};
