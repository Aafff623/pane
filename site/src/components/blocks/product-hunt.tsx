// Product Hunt featured badge — inline atom, parent controls layout/spacing.
// Used at the top of the homepage Hero and at the top of the global Footer.
import { withNavRef } from "@/lib/nav-ref";

export function ProductHunt() {
  return (
    <a
      href={withNavRef(
        "https://www.producthunt.com/products/design-mode?embed=true&utm_source=badge-featured&utm_medium=badge&utm_campaign=badge-design-mode-2",
      )}
      target="_blank"
      rel="noopener noreferrer"
      className="inline-block"
    >
      {/* Plain <img> — Next/image would need api.producthunt.com in
          images.remotePatterns, and PH re-stamps the SVG URL with a
          tracking param each render so caching would lie. */}
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img
        src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1146657&theme=light&t=1779274729933"
        alt="Design Mode — Featured on Product Hunt"
        width={250}
        height={54}
      />
    </a>
  );
}
