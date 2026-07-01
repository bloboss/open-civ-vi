// Generic tech/civics tree renderer. Tech and civics share an identical node
// shape, so one renderer drives both panels. Nodes are grouped by era; an
// `available` node is clickable to queue it.

import { clear, el } from "./dom.js";

/** Shared shape of TechNode / CivicNode from the generated bindings. */
export interface TreeNode {
  id: string;
  name: string;
  cost: number;
  progress: number | null;
  unlocks: string;
  era: string;
  status: string; // done | current | available | locked
  prereqs: string[];
}

export function renderTree(
  host: HTMLElement,
  nodes: TreeNode[],
  queue: string[],
  onPick: (id: string) => void,
): void {
  clear(host);

  const byId = new Map(nodes.map((n) => [n.id, n] as const));
  const queuedNames = queue.map((id) => byId.get(id)?.name ?? id);
  host.append(
    el("div", {
      class: "tree-queue muted",
      text: queue.length ? `Queue: ${queuedNames.join(" → ")}` : "Queue: (empty)",
    }),
  );

  // Group by era, preserving first-seen order.
  const eras: string[] = [];
  const byEra = new Map<string, TreeNode[]>();
  for (const n of nodes) {
    let bucket = byEra.get(n.era);
    if (!bucket) {
      bucket = [];
      byEra.set(n.era, bucket);
      eras.push(n.era);
    }
    bucket.push(n);
  }

  for (const era of eras) {
    host.append(el("div", { class: "tree-era", text: era }));
    for (const n of byEra.get(era)!) {
      const meta = n.progress !== null ? `${n.cost}◆ · ${n.progress}` : `${n.cost}◆`;
      const row = el("div", { class: `tree-node status-${n.status}` }, [
        el("span", { class: "tn-name", text: n.name, title: n.unlocks }),
        el("span", { class: "tn-meta muted", text: meta }),
        el("span", { class: `tn-status badge-${n.status}`, text: n.status }),
      ]);
      if (n.status === "available") {
        row.classList.add("clickable");
        row.append(
          el("button", {
            class: "btn tn-pick",
            text: "▸",
            title: `Choose ${n.name}`,
            onclick: () => onPick(n.id),
          }),
        );
      }
      host.append(row);
    }
  }
}
