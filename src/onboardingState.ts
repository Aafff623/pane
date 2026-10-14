/** A hidden first launch must retain its tour after the version marker advances. */
export function initialTourPending(state: {
  tourSeen: boolean;
  tourPending: boolean;
  lastSeenVersion: string;
  welcomeDismissed: boolean;
}): boolean {
  return !state.tourSeen && (state.tourPending || (!state.lastSeenVersion && !state.welcomeDismissed));
}
