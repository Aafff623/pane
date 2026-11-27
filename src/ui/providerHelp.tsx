import { useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import {
  Button, Collapsible, CollapsiblePanel, CollapsibleTrigger,
  Dialog, DialogClose, DialogDescription, DialogPopup, DialogTitle,
} from "./coss";
import "./coss.css";

type Result = { tone: "testing" | "ok" | "warn" | "bad"; text: string; detail?: string };
type ProviderHelpProps = {
  name: string;
  subtitle: string;
  badge: string;
  icon: string;
  closeIcon: string;
  infoIcon: string;
  chevronIcon: string;
  closeLabel: string;
  testLabel: string;
  testingLabel: string;
  stepsTitle: string;
  steps: string[];
  detailsTitle: string;
  principleTitle: string;
  principle: string;
  boundaryTitle: string;
  boundary: string;
  facts: { label: string; value: string; mono: boolean }[];
  links: { label: string; url: string }[];
  test: () => Promise<Result>;
};

function ProviderHelp({
  content, returnFocus, dispose,
}: {
  content: ProviderHelpProps;
  returnFocus: () => HTMLElement | null;
  dispose: () => void;
}) {
  const [open, setOpen] = useState(true);
  const [result, setResult] = useState<Result>();
  const testButton = useRef<HTMLButtonElement>(null);

  async function runTest() {
    setResult({ tone: "testing", text: content.testingLabel });
    setResult(await content.test());
  }

  return (
    <Dialog open={open} onOpenChange={setOpen} onOpenChangeComplete={(visible) => {
      if (!visible) queueMicrotask(dispose);
    }}>
      <DialogPopup
        id="confirm-box"
        className="provider-help"
        initialFocus={testButton}
        finalFocus={() => returnFocus() ?? true}
      >
        <header className="ph-head">
          <span className="ph-icon" aria-hidden="true" dangerouslySetInnerHTML={{ __html: content.icon }} />
          <div className="ph-heading">
            <DialogTitle className="ph-title">{content.name}</DialogTitle>
            <DialogDescription className="ph-sub">
              {content.subtitle}<span className="ph-sep">·</span>{content.badge}
            </DialogDescription>
          </div>
          <DialogClose aria-label={content.closeLabel} render={<Button variant="icon" />}>
            <span aria-hidden="true" dangerouslySetInnerHTML={{ __html: content.closeIcon }} />
          </DialogClose>
        </header>
        <div className="ph-body" tabIndex={0}>
          <dl className="ph-facts">
            {content.facts.map(({ label, value, mono }) => (
              <div key={label}><dt>{label}</dt><dd className={mono ? "mono" : undefined}>{value}</dd></div>
            ))}
          </dl>
          <section className="ph-section">
            <h4>{content.stepsTitle}</h4>
            <ol className="ph-steps">{content.steps.map((step) => <li key={step}>{step}</li>)}</ol>
          </section>
          <Collapsible className="ph-disclosure">
            <CollapsibleTrigger className="ph-disclosure-trigger">
              <span className="ph-disclosure-leading" aria-hidden="true" dangerouslySetInnerHTML={{ __html: content.infoIcon }} />
              <span className="ph-disclosure-label">{content.detailsTitle}</span>
              <span className="ph-disclosure-chevron" aria-hidden="true" dangerouslySetInnerHTML={{ __html: content.chevronIcon }} />
            </CollapsibleTrigger>
            <CollapsiblePanel>
              <div className="ph-disclosure-body">
                <h4>{content.principleTitle}</h4>
                <p>{content.principle}</p>
                <h4>{content.boundaryTitle}</h4>
                <p>{content.boundary}</p>
              </div>
            </CollapsiblePanel>
          </Collapsible>
          {content.links.length > 0 && (
            <div className="ph-links">
              {content.links.map(({ label, url }) => <a key={url} href={url} target="_blank" rel="noreferrer">{label} ↗</a>)}
            </div>
          )}
        </div>
        <div className="ph-actions">
          <div className={result ? "ph-status is-" + result.tone : "ph-status"} title={result?.detail} aria-live="polite">
            {result?.text}
          </div>
          <DialogClose render={<Button variant="secondary" />}>{content.closeLabel}</DialogClose>
          <Button ref={testButton} disabled={result?.tone === "testing"} onClick={() => { void runTest(); }}>
            {result?.tone === "testing" ? content.testingLabel : content.testLabel}
          </Button>
        </div>
      </DialogPopup>
    </Dialog>
  );
}

export function showProviderHelp(
  content: ProviderHelpProps,
  returnFocus: () => HTMLElement | null,
  onDispose: () => void,
) {
  const host = document.createElement("div");
  host.setAttribute("data-provider-help-host", "");
  document.body.append(host);
  const root = createRoot(host);
  const dispose = () => {
    if (!host.isConnected) return;
    root.unmount();
    host.remove();
    onDispose();
  };
  root.render(<ProviderHelp content={content} returnFocus={returnFocus} dispose={dispose} />);
  return dispose;
}
