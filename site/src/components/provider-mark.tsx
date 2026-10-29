import { cn } from "@/lib/utils";

const MARKS: Record<string, string> = {
  claude: "/img/providers/claude.svg",
  codex: "/img/providers/codex.svg",
  cursor: "/img/providers/cursor.svg",
  copilot: "/img/providers/copilot.svg",
  deepseek: "/img/providers/deepseek.svg",
  minimax: "/img/providers/minimax.svg",
  qoder: "/img/providers/qoder-cn.svg",
  openrouter: "/img/providers/openrouter.svg",
};

export function ProviderMark({ name, size = 20, className }: { name: keyof typeof MARKS | string; size?: number; className?: string }) {
  const src = MARKS[name] ?? name;
  return (
    <span
      aria-hidden="true"
      className={cn("inline-block bg-current text-foreground/80", className)}
      style={{
        width: size,
        height: size,
        maskImage: `url(${src})`,
        maskSize: "contain",
        maskRepeat: "no-repeat",
        maskPosition: "center",
        WebkitMaskImage: `url(${src})`,
        WebkitMaskSize: "contain",
        WebkitMaskRepeat: "no-repeat",
        WebkitMaskPosition: "center",
      }}
    />
  );
}
