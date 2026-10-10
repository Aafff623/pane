// Source: https://github.com/magicuidesign/magicui/blob/ec1cce6c4192c0aaac279dd7e53537ccd5c99d44/apps/www/registry/example/animated-grid-pattern-demo.tsx
// Revision: ec1cce6c4192c0aaac279dd7e53537ccd5c99d44
// Author: Dillion Verma / Magic UI
// License: MIT
// Path preserved from the original repository: apps/www/registry/example/animated-grid-pattern-demo.tsx

import { cn } from "@/lib/utils"
import { AnimatedGridPattern } from "@/registry/magicui/animated-grid-pattern"

export default function AnimatedGridPatternDemo() {
  return (
    <div className="bg-background relative flex h-[500px] w-full items-center justify-center overflow-hidden rounded-lg border p-20">
      <AnimatedGridPattern
        numSquares={30}
        maxOpacity={0.1}
        duration={3}
        repeatDelay={1}
        className={cn(
          "mask-[radial-gradient(500px_circle_at_center,white,transparent)]",
          "inset-x-0 inset-y-[-30%] h-[200%] skew-y-12"
        )}
      />
    </div>
  )
}
