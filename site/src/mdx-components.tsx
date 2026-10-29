import type { MDXComponents } from "mdx/types";

import { withNavRef } from "@/lib/nav-ref";

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    ...components,
    a: ({ href, children, ...props }) => {
      const nextHref = typeof href === "string" ? withNavRef(href) : href;
      const external =
        typeof nextHref === "string" && /^https?:\/\//i.test(nextHref);
      return (
        <a
          href={nextHref}
          {...(external ? { target: "_blank", rel: "noopener noreferrer" } : {})}
          {...props}
        >
          {children}
        </a>
      );
    },
  };
}
