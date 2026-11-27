import { createRoot, type Root } from "react-dom/client";
import { Radio as RadioPrimitive } from "@base-ui/react/radio";
import { RadioGroup as RadioGroupPrimitive } from "@base-ui/react/radio-group";
import "./coss.css";

// Segmented presentation of the coss Radio Group. See LICENSE.coss-ui.
type TimeFormat = "auto" | "12" | "24";
type TimeFormatProps = {
  label: string;
  value: TimeFormat;
  options: { value: TimeFormat; label: string }[];
  onChange: (value: TimeFormat) => void;
};

let root: Root | undefined;

export function renderTimeFormat(host: HTMLElement, { label, value, options, onChange }: TimeFormatProps) {
  root ??= createRoot(host);
  root.render(
    <RadioGroupPrimitive
      aria-label={label}
      value={value}
      onValueChange={onChange}
      data-slot="radio-group"
      className="tabs pane:inline-flex pane:max-w-full pane:flex-wrap pane:gap-1 pane:rounded-lg pane:bg-(--accent) pane:p-1"
    >
      {options.map((option) => (
        <RadioPrimitive.Root
          key={option.value}
          value={option.value}
          nativeButton
          render={<button type="button" />}
          data-slot="radio"
          className={({ checked }) => checked ? "tab active" : "tab"}
        >
          {option.label}
        </RadioPrimitive.Root>
      ))}
    </RadioGroupPrimitive>,
  );
}

