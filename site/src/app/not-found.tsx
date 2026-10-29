import Link from "next/link";

import { Background } from "@/components/background";
import { Button } from "@/components/ui/button";

export default function NotFound() {
  return (
    <Background>
      <section className="container flex min-h-[70vh] max-w-3xl flex-col items-start justify-center py-24">
        <p className="text-muted-foreground text-base font-semibold tracking-wide uppercase">
          404 · Page not found
        </p>
        <h1 className="mt-4 text-3xl tracking-tight sm:text-4xl md:text-5xl">
          This route is not part of Design Mode.
        </h1>
        <p className="text-muted-foreground mt-4 max-w-2xl text-base leading-relaxed">
          The link may be outdated. Start with the product workflow,
          installation guide or documentation index.
        </p>
        <div className="mt-8 flex flex-wrap gap-4">
          <Button asChild>
            <Link href="/">Go to the homepage</Link>
          </Button>
          <Button asChild variant="outline">
            <Link href="/docs">Browse documentation</Link>
          </Button>
        </div>
      </section>
    </Background>
  );
}
