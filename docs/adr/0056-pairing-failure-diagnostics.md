# ADR 0056 — Actionable pairing failures and invitation refresh

The user reported Windows rejecting an Android invitation or never reaching approval.
No device is attached to the development host and the exact on-device error is pending.
The previous UI suppressed the existing native failure number and displayed the same
generic message for malformed/expired invitations, connection errors and handshake failures.

Keep the existing scalar status ABI. Add stable diagnostic codes for invitation parsing,
expiry, clock-window rejection, TCP refusal/timeout, handshake, proof, response identity
and approval timeout. The UI displays only fixed text and the code. It never includes the
invitation, nonce, peer-supplied rejection strings or raw network errors in copied diagnostics.
TLS pinning, proof verification and explicit consent remain unchanged.

The pending client closes when the invitation's remaining validity expires. This avoids an
indefinite approval wait after the receiver can no longer approve the request.

Refreshing the receiver invitation now resumes the existing accept worker automatically.
A Java wait generation prevents callbacks from a cancelled older wait from changing the
state of the new wait. No approval is automated and no extra endpoints are scanned.

Builds and static analysis are permitted; no tests or workflows are added. These changes
improve the failure-reporting path but do not establish the cause of the user's device failure.
