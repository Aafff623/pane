# Magpie routing reference

Author: yetone. License: MIT (see LICENSE).

Local upstream snapshot: 290c60a31ebe701903af425cae6a95fbe1f55255, from `temp/magpie-yetone`.
Source: https://github.com/yetone/magpie

`routing.js` is the original GUI routing source; `site-original.html` contains its website simulation. Both are reference archives, not scripts served to the browser.

Pane's `public/usage-flow.js` adapts the measured Bézier connections, eased SVG path traversal, solid outbound/hollow return packets, node arrival pulse and 380/420/360/320 ms trip phases to sample usage collection. It does not implement Magpie's gateway routing, account fallback, or network requests. Each full trip is explicitly counted as a sample collection.
