# Ticket 01 · 外观入口与路由
Status: done — owner accepted 2026-10-27 | Priority: P1 | Depends: none

Build: In existing Settings → General add Appearance/皮肤工坊. Reuse #49 shell, not a new top-level window. Redirect old custom-icon/drawer workshop entry to Settings appearance without breaking keyboard navigation. Preserve existing config keys and old ability to restore native.

Done when: opening through General or legacy entry lands in same route; back preserves settings context; no duplicate skin market DOM and no extra vertical scrollbar. Review i18n and four skin shell compatibility, but don't modify skin CSS.
