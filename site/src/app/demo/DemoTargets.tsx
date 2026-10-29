import type { ReactElement } from "react";

import styles from "./demo.module.scss";

// Each demo target is a real, styled HTML block that the user edits via
// the extension. They carry stable `id` attributes so users can find them
// in the Layers tab. They do NOT carry `data-dm-*` attributes — those
// belong to the extension.

const PHOTO_URL = "/cover.png";

export const DEMO_TARGETS: Record<string, () => ReactElement> = {
  inspector: () => (
    <div id="demo-inspector-card" className={styles.targetCard}>
      <h3>Hover me. Then click me.</h3>
      <p>
        I'm a generic card waiting for you to pick me. Selecting locks the focus
        on me until you click somewhere else.
      </p>
    </div>
  ),

  "measure-resize": () => (
    <div id="demo-measure-card" className={styles.targetCard}>
      <h3>Select me, then resize me</h3>
      <p>
        Grab one of my 8 handles and drag. Hover the elements around me to read
        the edge-to-edge spacing pills.
      </p>
    </div>
  ),

  "multi-select": () => (
    <div id="demo-multi-row" className={styles.targetMultiRow}>
      <div className={styles.targetMultiCard}>One</div>
      <div className={styles.targetMultiCard}>Two</div>
      <div className={styles.targetMultiCard}>Three</div>
    </div>
  ),

  layers: () => (
    <ul id="demo-layers-list" className={styles.targetList}>
      <li>List item one — drag me</li>
      <li>List item two</li>
      <li>List item three</li>
      <li>List item four</li>
    </ul>
  ),

  "annotate-draw": () => (
    <div id="demo-annotate-card" className={styles.targetCard}>
      <h3>Pin a comment on me</h3>
      <p>
        Drop a comment pin here with Alt+C, then drag a region comment around me
        with Alt+R. Both stay as overlays — my own styles never change.
      </p>
    </div>
  ),

  "design-indicator": () => (
    <div id="demo-indicator-row" className={styles.targetIndicatorRow}>
      <svg
        id="demo-heart"
        className="lucide lucide-heart"
        xmlns="http://www.w3.org/2000/svg"
        width="32"
        height="32"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z" />
      </svg>
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img
        id="demo-photo"
        src={PHOTO_URL}
        alt="Design Mode's visual editor on a webpage"
        width={160}
        height={120}
        className={styles.targetPhoto}
      />
    </div>
  ),

  "design-position": () => (
    <div id="demo-position-stage" className={styles.targetStage}>
      <div id="demo-position-card" className={styles.targetPositionCard}>
        <strong>Position me</strong>
        <p>Use the alignment buttons in Position to pin me.</p>
      </div>
    </div>
  ),

  "design-layout": () => (
    <div id="demo-layout-container" className={styles.targetLayoutContainer}>
      <div className={styles.targetLayoutChild}>A</div>
      <div className={styles.targetLayoutChild}>B</div>
      <div className={styles.targetLayoutChild}>C</div>
      <div className={styles.targetLayoutChild}>D</div>
    </div>
  ),

  "design-appearance": () => (
    <div id="demo-appearance-stage" className={styles.targetAppearanceStage}>
      <div id="demo-appearance-card" className={styles.targetAppearanceCard}>
        Pick me. Tweak my opacity, blend mode, and corner radius.
      </div>
    </div>
  ),

  "design-typography": () => (
    <article id="demo-typography-block" className={styles.targetTypography}>
      <h3 id="demo-typography-heading">Sample heading</h3>
      <p id="demo-typography-paragraph">
        Pick this paragraph and try changing the line-height, letter-spacing,
        weight, or font family.
      </p>
      <ul id="demo-typography-list">
        <li>Bulleted item one</li>
        <li>Bulleted item two</li>
        <li>Bulleted item three</li>
      </ul>
    </article>
  ),

  "design-fill": () => (
    <div id="demo-fill-hero" className={styles.targetFillHero}>
      <strong>Fill me</strong>
      <p>Add a linear gradient on top of my solid background.</p>
    </div>
  ),

  "design-stroke": () => (
    <div id="demo-stroke-square" className={styles.targetStrokeSquare}>
      No stroke yet
    </div>
  ),

  "design-effects": () => (
    <div id="demo-effects-stage" className={styles.targetEffectsStage}>
      <div id="demo-effects-card" className={styles.targetEffectsCard}>
        Drop a shadow on me
      </div>
    </div>
  ),

  "design-motion": () => (
    <div id="demo-motion-stage" className={styles.targetMotionStage}>
      <div id="demo-motion-card" className={styles.targetMotionCard}>
        <strong>Animate me</strong>
        <p>
          Pick me, expand Motion, add a Transition on background-color or
          transform, then hover this card.
        </p>
      </div>
    </div>
  ),

  "design-layout-guide": () => (
    <div
      id="demo-layout-guide-container"
      className={styles.targetLayoutGuideContainer}
    >
      <span className={styles.targetLayoutGuideHint}>
        Wide container — apply a Columns × 12 layout guide on me.
      </span>
    </div>
  ),

  "design-tokens": () => (
    <div id="demo-tokens-stage" className={styles.targetTokensStage}>
      <div id="demo-tokens-swatch" className={styles.targetTokensSwatch} />
      <span className={styles.targetTokensHint}>
        Open my Fill colour picker — pick from the Tokens row.
      </span>
    </div>
  ),

  presets: () => (
    <div id="demo-presets-card" className={styles.targetPresetCard}>
      <strong>Style me, then save me as a preset</strong>
      <p>Apply a seeded preset, or save my look to reuse on another element.</p>
    </div>
  ),

  "action-row": () => (
    <div id="demo-action-stage" className={styles.targetActionStage}>
      <div id="demo-action-group" className={styles.targetActionGroup}>
        <span className={styles.targetActionLabel}>Parent group</span>
        <div
          id="demo-pulse-badge"
          className={styles.targetPulse}
          title="Select me, then try Parent / Child, Duplicate, Remove, and Pause animations in the action row"
        >
          ✦ pulsing badge
        </div>
        <span className={styles.targetActionSibling}>sibling</span>
      </div>
    </div>
  ),
};

export function renderDemoTarget(targetId?: string): React.ReactNode {
  if (!targetId) return null;
  const Target = DEMO_TARGETS[targetId];
  if (!Target) return null;
  return <Target />;
}
