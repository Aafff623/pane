import { providerDefinition } from "./providerCatalog.ts";

export type UsageUnit = "tokens" | "credits" | "minutes" | "requests" | "characters" | "pages";

export interface MeterTrend {
  id: string;
  unit: UsageUnit;
  trend: (number | null)[];
}

export function primaryMeterTrend(id: string, meters: readonly MeterTrend[]): MeterTrend | undefined {
  const primary = providerDefinition(id.split("@")[0])?.primaryUnit ?? "tokens";
  const order: UsageUnit[] = [...new Set<UsageUnit>([primary, "tokens", "credits", "minutes", "requests", "characters", "pages"])];
  for (const unit of order) {
    const meter = meters.find(row => row.id === id && row.unit === unit && row.trend.some(value => value !== null));
    if (meter) return meter;
  }
  return undefined;
}
