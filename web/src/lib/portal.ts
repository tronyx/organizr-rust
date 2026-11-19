// Moves an element to the end of the page.
//
// A dialog that opens from inside a form must not be a part of that form: the Enter key
// in one of its fields would send the form around it. Use it as `use:portal`.

export function portal(node: HTMLElement): { destroy: () => void } {
  document.body.append(node);
  return {
    destroy() {
      node.remove();
    },
  };
}
