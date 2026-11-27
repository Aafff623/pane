// Selected components adapted from https://coss.com/ui/r/ on 2026-11-26.
// See LICENSE.coss-ui for upstream terms.
import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import { Collapsible as CollapsiblePrimitive } from "@base-ui/react/collapsible";
import { useRender } from "@base-ui/react/use-render";
import { mergeProps } from "@base-ui/react/merge-props";

export const Dialog = DialogPrimitive.Root;
export const DialogClose = DialogPrimitive.Close;
export const DialogTitle = DialogPrimitive.Title;
export const DialogDescription = DialogPrimitive.Description;

export function DialogPopup({ children, ...props }: DialogPrimitive.Popup.Props) {
  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Backdrop
        data-slot="dialog-backdrop"
        className="pane:fixed pane:inset-0 pane:z-[90] pane:bg-black/45 pane:backdrop-blur-sm pane:motion-safe:transition-opacity pane:data-ending-style:opacity-0 pane:data-starting-style:opacity-0"
      />
      <DialogPrimitive.Viewport
        data-slot="dialog-viewport"
        className="pane:fixed pane:inset-0 pane:z-[90] pane:grid pane:place-items-center pane:p-2"
      >
        <DialogPrimitive.Popup data-slot="dialog-popup" {...props}>
          {children}
        </DialogPrimitive.Popup>
      </DialogPrimitive.Viewport>
    </DialogPrimitive.Portal>
  );
}

export function Button({
  variant = "primary",
  render,
  ...props
}: useRender.ComponentProps<"button"> & { variant?: "primary" | "secondary" | "icon" }) {
  const variants = {
    primary: "pane:bg-(--primary) pane:text-(--primary-foreground) pane:px-3 pane:hover:brightness-110",
    secondary: "pane:bg-transparent pane:text-(--foreground) pane:px-3 pane:hover:bg-(--accent)",
    icon: "pane:w-8 pane:bg-transparent pane:p-0 pane:text-(--muted-foreground) pane:hover:bg-(--accent) pane:hover:text-(--foreground)",
  };
  const buttonProps = {
    type: "button" as const,
    "data-slot": "button",
    className: "pane:inline-flex pane:h-8 pane:shrink-0 pane:cursor-pointer pane:items-center pane:justify-center pane:rounded-md pane:border-0 pane:font-[inherit] pane:text-sm pane:font-medium pane:focus-visible:outline-2 pane:focus-visible:outline-offset-2 pane:focus-visible:outline-(--primary) pane:disabled:cursor-default pane:disabled:opacity-60 " + variants[variant],
  };
  return useRender({
    defaultTagName: "button",
    render,
    props: mergeProps<"button">(buttonProps, props),
  });
}

export const Collapsible = CollapsiblePrimitive.Root;
export const CollapsibleTrigger = CollapsiblePrimitive.Trigger;

export function CollapsiblePanel(props: CollapsiblePrimitive.Panel.Props) {
  return (
    <CollapsiblePrimitive.Panel
      data-slot="collapsible-panel"
      className="pane:h-(--collapsible-panel-height) pane:overflow-hidden pane:motion-safe:transition-[height] pane:data-ending-style:h-0 pane:data-starting-style:h-0"
      {...props}
    />
  );
}

