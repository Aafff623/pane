# 02 · Credential status and API fields
Status: done — implemented 2026-10-28; owner acceptance pending | Priority: P1 | Depends: 01

Build: extend `get_credential_status` with `installState`, `installEvidence`, `credentialSource`, `envVars`; add additive fields to `/v1/providers`; add `/v1/doctor` `installed_but_not_signed_in` with remediation; document the additive schema.
Done when: existing fields stay compatible; no secret values appear in either API; doctor check has a remediation when it fails.
