"use client";

import { Fragment, useState } from "react";

import { Check, ChevronsUpDown, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";

interface FeatureSection {
  category: string;
  features: {
    name: string;
    local: true | false | null | string;
    cloud: true | false | null | string;
    selfHosted: true | false | null | string;
  }[];
}

const pricingPlans = [
  { name: "Cloud" },
  { name: "Local" },
  { name: "Self-hosted" },
];

const comparisonFeatures: FeatureSection[] = [
  {
    category: "Setup",
    features: [
      {
        name: "Setup requirements",
        local: "checkout + dependencies + client stdio config",
        cloud: "bearer credential + HTTP client config",
        selfHosted: "Node.js + Redis relay + client config",
      },
      {
        name: "Client HTTP bearer header",
        local: false,
        cloud: true,
        selfHosted: true,
      },
      {
        name: "Reconnects on activation after setup",
        local: true,
        cloud: true,
        selfHosted: true,
      },
    ],
  },
  {
    category: "Privacy",
    features: [
      {
        name: "Design Mode MCP transport",
        local: "localhost bridge",
        cloud: "hosted relay",
        selfHosted: "your relay deployment",
      },
      {
        name: "Relay payload buffering",
        local: false,
        cloud: "60-second expiry requested",
        selfHosted: "operator-controlled",
      },
      {
        name: "Payload queue expiry",
        local: "n/a",
        cloud: "new messages refresh the queue expiry",
        selfHosted: "source default refreshes; verify your deployment",
      },
      {
        name: "Anyone else operates the infra",
        local: false,
        cloud: true,
        selfHosted: false,
      },
    ],
  },
  {
    category: "Agent compatibility",
    features: [
      {
        name: "Claude Desktop",
        local: "local stdio configuration",
        cloud: "verify custom bearer-header support",
        selfHosted: "verify custom bearer-header support",
      },
      {
        name: "Cursor",
        local: "stdio configuration",
        cloud: "HTTP + bearer header",
        selfHosted: "HTTP + bearer header",
      },
      {
        name: "Claude Code",
        local: "stdio configuration",
        cloud: "HTTP + bearer header",
        selfHosted: "HTTP + bearer header",
      },
      {
        name: "Agents in remote / sandboxed contexts",
        local: "requires access to the local bridge",
        cloud: "requires permitted authenticated HTTP",
        selfHosted: "requires permitted authenticated HTTP",
      },
    ],
  },
  {
    category: "Cost",
    features: [
      {
        name: "Price",
        local: "MIT-licensed",
        cloud: "currently no charge",
        selfHosted: "hosting costs apply",
      },
    ],
  },
];

const renderFeatureValue = (value: true | false | null | string) => {
  if (value === true) {
    return (
      <span className="inline-flex items-center gap-2">
        <Check className="size-6" aria-hidden="true" />
        <span className="sr-only">Yes</span>
      </span>
    );
  }
  if (value === false) {
    return (
      <span className="inline-flex items-center gap-2">
        <X className="size-6" aria-hidden="true" />
        <span className="sr-only">No</span>
      </span>
    );
  }
  if (value === null) {
    return null;
  }
  return (
    <div className="flex items-center gap-2">
      <Check className="size-4" />
      <span className="text-muted-foreground">{value}</span>
    </div>
  );
};

export const ModesComparison = () => {
  const [selectedPlan, setSelectedPlan] = useState(0); // Default to Cloud mode

  return (
    <section className="pb-28 lg:py-32" aria-labelledby="mcp-mode-comparison">
      <div className="container">
        <h2 id="mcp-mode-comparison" className="sr-only">
          MCP connection mode comparison
        </h2>

        <div className="md:hidden">
          <MobilePlanSelector
            selectedPlan={selectedPlan}
            onPlanChange={setSelectedPlan}
          />
          {comparisonFeatures.map((section) => (
            <section
              key={section.category}
              aria-labelledby={`mobile-${section.category}`}
            >
              <h3
                id={`mobile-${section.category}`}
                className="border-primary/40 border-b py-4 text-2xl font-semibold"
              >
                {section.category}
              </h3>
              <dl>
                {section.features.map((feature) => {
                  const value = [
                    feature.cloud,
                    feature.local,
                    feature.selfHosted,
                  ][selectedPlan];

                  return (
                    <div
                      key={feature.name}
                      className="grid grid-cols-2 border-b font-medium"
                    >
                      <dt className="py-4">{feature.name}</dt>
                      <dd
                        className="flex items-center py-4"
                        aria-label={`${pricingPlans[selectedPlan].name}: ${value === true ? "Yes" : value === false ? "No" : (value ?? "Not applicable")}`}
                      >
                        {renderFeatureValue(value)}
                      </dd>
                    </div>
                  );
                })}
              </dl>
            </section>
          ))}
        </div>

        <div className="hidden overflow-x-auto md:block">
          <table className="w-full table-fixed text-left">
            <caption className="sr-only">
              Compare Cloud, Local and Self-hosted Design Mode MCP connection
              modes
            </caption>
            <thead>
              <tr>
                <th scope="col" className="w-1/4 py-4 font-semibold">
                  Feature
                </th>
                {pricingPlans.map((plan) => (
                  <th
                    key={plan.name}
                    scope="col"
                    className="py-4 text-2xl font-semibold"
                  >
                    {plan.name}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {comparisonFeatures.map((section) => (
                <Fragment key={section.category}>
                  <tr>
                    <th
                      scope="colgroup"
                      colSpan={4}
                      className="border-primary/40 border-b py-4 text-2xl font-semibold"
                    >
                      {section.category}
                    </th>
                  </tr>
                  {section.features.map((feature) => (
                    <tr key={feature.name} className="font-medium">
                      <th scope="row" className="border-b py-4 pr-4">
                        {feature.name}
                      </th>
                      {[feature.cloud, feature.local, feature.selfHosted].map(
                        (value, index) => (
                          <td
                            key={pricingPlans[index].name}
                            className="border-b py-4 pr-4"
                          >
                            {renderFeatureValue(value)}
                          </td>
                        ),
                      )}
                    </tr>
                  ))}
                </Fragment>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </section>
  );
};

const MobilePlanSelector = ({
  selectedPlan,
  onPlanChange,
}: {
  selectedPlan: number;
  onPlanChange: (index: number) => void;
}) => {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <div className="flex items-center justify-between border-b py-4">
        <CollapsibleTrigger className="flex items-center gap-2">
          <span className="text-2xl font-semibold">
            {pricingPlans[selectedPlan].name}
          </span>
          <ChevronsUpDown
            className={`size-6 transition-transform ${isOpen ? "rotate-180" : ""}`}
            aria-hidden="true"
          />
        </CollapsibleTrigger>
      </div>
      <CollapsibleContent className="flex flex-col space-y-2 p-2">
        {pricingPlans.map(
          (plan, index) =>
            index !== selectedPlan && (
              <Button
                size="lg"
                variant="secondary"
                key={plan.name}
                onClick={() => {
                  onPlanChange(index);
                  setIsOpen(false);
                }}
              >
                {plan.name}
              </Button>
            ),
        )}
      </CollapsibleContent>
    </Collapsible>
  );
};
