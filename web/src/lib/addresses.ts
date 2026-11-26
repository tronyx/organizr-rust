// Addresses that the interface puts into a frame or behind a link.
//
// An Admin writes the address of a tab, and the PHP application stores any text. A
// browser runs an address such as `javascript:alert(1)` as a script on this site, so
// the interface never opens one. Other schemes, such as `steam://`, keep working.

/** The schemes whose address a browser runs as a script or shows as a page of its own. */
const SCRIPT_SCHEMES = ['javascript', 'data', 'vbscript'];

/**
 * Whether an address has a scheme that runs a script. The address is read as a browser
 * reads it: control characters and spaces at the start are left out, and so are tabs
 * and line breaks anywhere.
 */
export function runsScript(address: string): boolean {
  let start = 0;
  while (start < address.length && address.charCodeAt(start) <= 0x20) {
    start += 1;
  }
  const cleaned = address.slice(start).replace(/[\t\n\r]/g, '');
  const scheme = /^([a-z][a-z0-9+.-]*):/i.exec(cleaned)?.[1];
  return scheme !== undefined && SCRIPT_SCHEMES.includes(scheme.toLowerCase());
}

/** The address, when the interface may open it: not empty, and no script. */
export function safeAddress(address: string | null | undefined): string | undefined {
  return address && !runsScript(address) ? address : undefined;
}
