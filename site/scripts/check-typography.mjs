import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

const { chromium } = await import(
  process.env.PLAYWRIGHT_MODULE
    ? pathToFileURL(process.env.PLAYWRIGHT_MODULE).href
    : "playwright"
);
const base = process.env.BASE_URL || "http://localhost:3127";
const output = process.env.TYPOGRAPHY_OUTPUT || "typography-evidence";
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
  for (const width of [320, 360, 400, 421, 480, 640, 1440]) {
    for (const route of routes) {
      const page = await browser.newPage({
        viewport: { width, height: 900 },
        reducedMotion: "reduce",
      });
      const response = await page.goto(base + route);
      assert.equal(response.status(), 200, route);
      await page.evaluate(() => document.fonts.ready);
      const result = await page.evaluate(() => {
        const exempt = (element) =>
          element.closest(
            '[role="img"], [class*="_target"], [class*="_stepTarget"]',
          );
        const visible = (element) =>
          element.getClientRects().length &&
          getComputedStyle(element).visibility !== "hidden";
        const groups = {
          body: "p, li, blockquote, figcaption, pre, code, th, td",
          buttons: 'button, [data-slot="button"], [data-button-content]',
          navigation: "header nav a, footer li a",
          labels: ".text-base, #copy-prompt-status",
        };
        const counts = {};
        const failures = [];
        for (const [name, selector] of Object.entries(groups)) {
          const elements = [...document.querySelectorAll(selector)].filter(
            (e) =>
              visible(e) &&
              !exempt(e) &&
              (name === "buttons" || !e.closest("h1,h2,h3,h4,h5,h6")),
          );
          counts[name] = elements.length;
          for (const e of elements)
            if (getComputedStyle(e).fontSize !== "16px")
              failures.push({
                name,
                tag: e.tagName,
                class: e.className,
                text: e.textContent.trim().slice(0, 70),
                size: getComputedStyle(e).fontSize,
              });
        }
        const fontFailures = [...document.querySelectorAll("body *")]
          .filter(
            (e) =>
              visible(e) &&
              e.childNodes.length &&
              [...e.childNodes].some(
                (n) => n.nodeType === Node.TEXT_NODE && n.textContent.trim(),
              ) &&
              !e.closest("svg, script, style") &&
              !/inter/i.test(getComputedStyle(e).fontFamily),
          )
          .map((e) => ({
            tag: e.tagName,
            text: e.textContent.slice(0, 60),
            font: getComputedStyle(e).fontFamily,
          }));
        const externalFonts = performance
          .getEntriesByType("resource")
          .filter((r) => /fonts\.(googleapis|gstatic)\.com/.test(r.name))
          .map((r) => r.name);
        const h1 = document.querySelector("h1");
        return {
          counts,
          failures,
          fontFailures,
          externalFonts,
          font: getComputedStyle(document.body).fontFamily,
          overflow: document.documentElement.scrollWidth - innerWidth,
          h1: h1 && getComputedStyle(h1).fontSize,
          brokenImages: [...document.images]
            .filter((i) => i.complete && !i.naturalWidth)
            .map((i) => i.src),
        };
      });
      assert.ok(
        result.counts.body > 0,
        `${route}: body selectors must resolve`,
      );
      assert.ok(
        result.counts.buttons > 0,
        `${route}: button selectors must resolve`,
      );
      results.push({ route, width, ...result });
      if (
        [320, 1440].includes(width) &&
        ["/", "/mcp", "/privacy", "/docs/mcp-setup", "/demo"].includes(route)
      )
        await page.screenshot({
          path: `${output}/${route.replaceAll("/", "_") || "home"}-${width}.png`,
          fullPage: false,
        });
      await page.close();
    }
  }
  const interactions = [];
  for (const width of [320, 1440]) {
    const page = await browser.newPage({
      viewport: { width, height: 900 },
      reducedMotion: "reduce",
    });
    await page.goto(base + "/");
    const heroButtons = await page
      .locator('main [data-slot="button"]')
      .evaluateAll((elements) =>
        elements.slice(0, 2).map((e) => ({
          size: getComputedStyle(e).fontSize,
          height: e.getBoundingClientRect().height,
        })),
      );
    assert.equal(heroButtons.length, 2);
    heroButtons.forEach((button) => {
      assert.equal(button.size, "16px");
      assert.ok(button.height >= 56);
    });
    await page.locator("#panel-anatomy-title").scrollIntoViewIfNeeded();
    const stage =
      width < 1024
        ? page.locator('#panel-anatomy button[aria-label="Header"]')
        : page
            .locator("#panel-anatomy button[aria-expanded]")
            .filter({ hasText: "Header" });
    await stage.click();
    await page.locator('[data-notes="Header"][data-ready="true"]').waitFor();
    const panelFonts = await page
      .locator(
        '#panel-anatomy [role="img"], #panel-anatomy [data-notes], #panel-anatomy [data-notes] button',
      )
      .evaluateAll((elements) =>
        elements.map((e) => ({
          font: getComputedStyle(e).fontFamily,
          size: getComputedStyle(e).fontSize,
        })),
      );
    assert.ok(panelFonts.length >= 2);
    panelFonts.forEach((sample) => assert.match(sample.font, /Inter/i));
    await page.screenshot({ path: `${output}/panel-${width}.png` });
    await page.locator("footer").scrollIntoViewIfNeeded();
    await page.screenshot({ path: `${output}/footer-${width}.png` });
    await page.goto(base + "/mcp");
    const triggers = page.locator("main button[aria-expanded]");
    let opened = 0;
    for (let index = 0; index < (await triggers.count()); index++) {
      const trigger = triggers.nth(index);
      if (!(await trigger.isVisible())) continue;
      if ((await trigger.getAttribute("aria-expanded")) === "false")
        await trigger.click();
      assert.equal(
        await trigger.evaluate((e) => getComputedStyle(e).fontSize),
        "16px",
      );
      opened++;
    }
    assert.ok(opened > 0, "MCP accordions must be exercised");
    const expandedFailures = await page
      .locator('[role="region"] p, [role="region"] pre, [role="region"] code')
      .evaluateAll((elements) =>
        elements
          .filter(
            (e) =>
              e.getClientRects().length &&
              getComputedStyle(e).fontSize !== "16px",
          )
          .map((e) => e.textContent),
      );
    assert.deepEqual(expandedFailures, []);
    await page.locator("#tools-title").scrollIntoViewIfNeeded();
    await page.screenshot({ path: `${output}/mcp-tools-${width}.png` });
    interactions.push({
      width,
      heroButtons,
      panelFonts,
      opened,
      expandedFailures,
    });
    await page.close();
  }
  const page = await browser.newPage({
    viewport: { width: 1440, height: 900 },
  });
  await page.goto(base + "/mcp");
  await page.evaluate(() => {
    document.documentElement.style.fontSize = "32px";
  });
  const enlarged = await page
    .locator("#copy-prompt-status")
    .evaluate((e) => getComputedStyle(e).fontSize);
  assert.equal(
    enlarged,
    "32px",
    "Body typography follows user root-font scaling",
  );
  await writeFile(
    `${output}/results.json`,
    JSON.stringify(
      { rootFont200Percent: enlarged, interactions, results },
      null,
      2,
    ),
  );
  const failed = results.filter(
    (r) =>
      r.failures.length ||
      r.fontFailures.length ||
      r.externalFonts.length ||
      r.overflow ||
      r.brokenImages.length ||
      !r.h1 ||
      parseFloat(r.h1) <= 16,
  );
  console.log(JSON.stringify({ samples: results.length, failed }, null, 2));
  assert.equal(
    failed.length,
    0,
    "Rendered typography, heading hierarchy, images and reflow",
  );
} finally {
  await browser.close();
}
