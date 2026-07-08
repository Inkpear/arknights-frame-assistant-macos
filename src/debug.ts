/** Debug utility — prefix all IPC logs for easy filtering in Console.app */
const PREFIX = "[AFA]";

export function logIpc(direction: "→" | "←", command: string, payload?: unknown) {
  console.debug(`${PREFIX} IPC ${direction} ${command}`, payload ?? "");
}

export function logState(label: string, detail?: unknown) {
  console.debug(`${PREFIX} STATE ${label}`, detail ?? "");
}

export function logUI(label: string) {
  console.debug(`${PREFIX} UI ${label}`);
}
