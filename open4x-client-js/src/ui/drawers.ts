// HUD drawers — notifications (with dismiss), the turn queue, and map overlays.
// Read-mostly; notifications supports dismiss / clear-all. Overlay toggles are
// local-state only for now (applying them in the WebGL renderer is a follow-up).

import type { MapOverlays, Notifications, TurnQueue } from "../gen/protocol/protocol.js";
import { clear, el } from "./dom.js";

export interface NotificationHandlers {
  onDismiss: (id: string) => void;
  onClearAll: () => void;
}

export function renderNotifications(
  host: HTMLElement,
  data: Notifications,
  h: NotificationHandlers,
): void {
  clear(host);
  host.append(
    el("div", { class: "drawer-head" }, [
      el("span", { text: `Notifications · turn ${data.turn}` }),
      el("button", { class: "btn", text: "clear all", onclick: () => h.onClearAll() }),
    ]),
  );
  if (data.notifications.length === 0) {
    host.append(el("p", { class: "muted", text: "(none)" }));
    return;
  }
  for (const n of data.notifications) {
    host.append(
      el("div", { class: `notif kind-${n.kind}` }, [
        el("div", { class: "notif-main" }, [
          el("span", { class: "notif-title", text: n.title }),
          ...(n.desc ? [el("span", { class: "muted notif-desc", text: n.desc })] : []),
        ]),
        el("button", { class: "btn", text: "✕", title: "dismiss", onclick: () => h.onDismiss(n.id) }),
      ]),
    );
  }
}

export function renderTurnQueue(host: HTMLElement, data: TurnQueue): void {
  clear(host);
  host.append(el("div", { class: "drawer-head", text: `Turn Queue · turn ${data.turn}` }));
  if (data.items.length === 0) {
    host.append(el("p", { class: "muted", text: "(empty)" }));
    return;
  }
  for (const it of data.items) {
    host.append(
      el("div", { class: "tq-item" }, [
        el("span", {
          class: it.required ? "badge-req" : "badge-opt",
          text: it.required ? "required" : "optional",
        }),
        el("span", { class: "tq-title", text: it.title }),
      ]),
    );
  }
}

export interface OverlayHandlers {
  onToggle: (id: string) => void;
}

export function renderOverlays(
  host: HTMLElement,
  data: MapOverlays,
  activeIds: Set<string>,
  h: OverlayHandlers,
): void {
  clear(host);
  host.append(el("div", { class: "drawer-head", text: "Map Overlays" }));
  if (data.overlays.length === 0) {
    host.append(el("p", { class: "muted", text: "(none)" }));
    return;
  }
  for (const o of data.overlays) {
    const on = activeIds.has(o.id);
    host.append(
      el("div", { class: `ovl-row${on ? " on" : ""}`, onclick: () => h.onToggle(o.id) }, [
        el("span", { text: on ? "☑" : "☐" }),
        el("span", { text: o.label }),
      ]),
    );
  }
}
