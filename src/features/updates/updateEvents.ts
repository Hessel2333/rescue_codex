export const requestUpdateCheckEvent = "rescue_codex:request-update-check";
export const updateCheckResultEvent = "rescue_codex:update-check-result";

export type UpdateCheckResult = {
  status: "checking" | "latest" | "available" | "error";
  version?: string;
};

export function requestUpdateCheck() {
  window.dispatchEvent(new Event(requestUpdateCheckEvent));
}

export function dispatchUpdateCheckResult(result: UpdateCheckResult) {
  window.dispatchEvent(new CustomEvent<UpdateCheckResult>(updateCheckResultEvent, { detail: result }));
}
