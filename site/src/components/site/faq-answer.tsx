import { Fragment } from "react";

import Link from "next/link";

import type { QA } from "@/content/product-facts";

export function FaqAnswer({ qa }: { qa: QA }) {
  return (
    <>
      {qa.answer}
      {qa.links?.length ? (
        <>
          {" "}
          {qa.links.map((link, index) => (
            <Fragment key={link.href}>
              {index > 0 && " · "}
              <Link href={link.href} className="underline underline-offset-4">
                {link.label}
              </Link>
            </Fragment>
          ))}
        </>
      ) : null}
    </>
  );
}
