import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

const { chromium } = await import(
  pathToFileURL(process.env.PLAYWRIGHT_MODULE).href
);
const output = process.env.VISUAL_OUTPUT;
const baseline = process.env.VISUAL_BASELINE;
const base = process.env.BASE_URL || "http://127.0.0.1:3134";
const routes = [
  "/",
  "/mcp",
  "/features",
  "/about",
  "/faq",
  "/contact",
  "/changelog",
  "/privacy",
  "/demo",
  "/docs",
  "/docs/mcp-setup",
  "/blog",
  "/blog/turn-visual-edits-into-precise-ai-prompts",
  "/compare",
  "/compare/design-mode-vs-magicpath",
  "/use-cases",
  "/use-cases/visual-editing-with-cursor",
];
const browser = await chromium.launch({
  headless: true,
  executablePath: process.env.CHROME_EXECUTABLE,
});
await mkdir(output, { recursive: true });
const results = [];
try {
  for (const width of [320, 360, 400, 404, 421, 480, 640, 1440]) {
    for (const route of routes) {
      const page = await browser.newPage({
        viewport: { width, height: 900 },
        reducedMotion: "reduce",
      });
      const response = await page.goto(base + route);
      assert.equal(response.status(), 200, route);
      await page.evaluate(() => document.fonts.ready);
      const measured = await page.evaluate(() => {
        const h1 = document.querySelector("h1");
        const style = (e) => {
          const s = getComputedStyle(e);
          return {
            size: s.fontSize,
            lineHeight: s.lineHeight,
            weight: s.fontWeight,
            tracking: s.letterSpacing,
            family: s.fontFamily,
          };
        };
        return {
          hero: h1 && [h1, ...h1.querySelectorAll("span")].map(style),
          h1Count: document.querySelectorAll("h1").length,
          overflow: document.documentElement.scrollWidth - innerWidth,
          headings: [...document.querySelectorAll("h2")].map((e) => ({
            text: e.textContent.trim(),
            ...style(e),
          })),
          bodyFailures: [
            ...document.querySelectorAll("main p, main li, main blockquote"),
          ]
            .filter(
              (e) =>
                e.getClientRects().length &&
                !e.closest(
                  '[role="img"], [class*="_target"], [class*="_stepTarget"]',
                ) &&
                getComputedStyle(e).fontSize !== "16px",
            )
            .map((e) => ({ text: e.textContent.slice(0, 60), ...style(e) })),
          brokenImages: [...document.images]
            .filter((i) => i.complete && !i.naturalWidth)
            .map((i) => i.src),
          buttons: [...document.querySelectorAll('main [data-slot="button"]')]
            .slice(0, 2)
            .map((e) => ({
              ...style(e),
              height: e.getBoundingClientRect().height,
              overflow: e.scrollWidth > e.clientWidth,
            })),
        };
      });
      results.push({ route, width, ...measured });
      if (
        [320, 1440].includes(width) &&
        ["/", "/mcp", "/privacy", "/docs/mcp-setup", "/demo"].includes(route)
      ) {
        await page.screenshot({
          path: `${output}/${route.replaceAll("/", "_")}-${width}.png`,
        });
        if (route === "/") {
          await page
            .locator("#agent-workflow-heading")
            .scrollIntoViewIfNeeded();
          await page.screenshot({ path: `${output}/workflow-${width}.png` });
          await page.locator("footer").scrollIntoViewIfNeeded();
          await page.screenshot({ path: `${output}/footer-${width}.png` });
        }
      }
      await page.close();
    }
  }
  await writeFile(`${output}/results.json`, JSON.stringify(results, null, 2));
  if (baseline) {
    const before = JSON.parse(await readFile(baseline, "utf8"));
    assert.equal(before.length, results.length);
    results.forEach((r, i) =>
      assert.deepEqual(
        r.hero,
        before[i].hero,
        `${r.route} ${r.width}: hero typography preserved`,
      ),
    );
    for (const r of results.filter(
      (r) => r.route === "/" && [320, 1440].includes(r.width),
    )) {
      const sections = r.headings.filter((h) =>
        [
          "Show your agent",
          "Every design control",
          "Questions before",
          "Start in your browser",
        ].some((text) => h.text.startsWith(text)),
      );
      assert.equal(sections.length, 4);
      sections.forEach((h) =>
        assert.equal(h.size, r.width === 320 ? "24px" : "48px"),
      );
      r.buttons.forEach((b) => assert.ok(b.height >= 56 && !b.overflow));
    }
    const failures = results.filter(
      (r) =>
        r.overflow ||
        r.h1Count !== 1 ||
        r.brokenImages.length ||
        r.bodyFailures.length ||
        r.buttons.some((b) => b.overflow),
    );
    console.log(
      JSON.stringify(
        { samples: results.length, heroEquality: true, failures },
        null,
        2,
      ),
    );
    assert.equal(failures.length, 0);
  } else
    console.log(
      JSON.stringify(
        {
          samples: results.length,
          baseline: true,
          overflow: results.filter((r) => r.overflow),
        },
        null,
        2,
      ),
    );
} finally {
  await browser.close();
}
