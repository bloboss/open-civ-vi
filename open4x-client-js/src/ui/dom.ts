// Tiny dependency-free DOM helpers — no framework, no innerHTML.

export interface Attrs {
  class?: string;
  text?: string;
  title?: string;
  onclick?: (e: MouseEvent) => void;
}

/** Create an element with optional class/text/click and children. */
export function el(tag: string, attrs: Attrs = {}, children: (Node | string)[] = []): HTMLElement {
  const node = document.createElement(tag);
  if (attrs.class) node.className = attrs.class;
  if (attrs.text !== undefined) node.textContent = attrs.text;
  if (attrs.title) node.title = attrs.title;
  if (attrs.onclick) node.addEventListener("click", attrs.onclick);
  for (const c of children) node.append(c);
  return node;
}

/** Remove all children of a node. */
export function clear(node: HTMLElement): void {
  node.replaceChildren();
}
