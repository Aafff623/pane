# Magic UI Animated Beam reference

Author: Dillion Verma / Magic UI. License: MIT (see LICENSE).

Upstream revision: ec1cce6c4192c0aaac279dd7e53537ccd5c99d44.
Source: https://github.com/magicuidesign/magicui/tree/ec1cce6c4192c0aaac279dd7e53537ccd5c99d44/apps/www/registry

The original `animated-beam.tsx` and `animated-beam-multiple-outputs.tsx` are archived unchanged from the user-supplied source bundle. The site's `public/usage-flow.js` adapts SVG gradient coordinate animation to native JavaScript, retaining the orange/purple stops, 3-second example duration and cubic-bezier(0.16, 1, 0.3, 1) easing. Node content, path geometry and stagger are adapted to Pane's usage diagram. React, Motion and Tailwind are not runtime dependencies of this static site.
