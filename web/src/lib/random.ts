// Random text from the random numbers of the browser that are fit for secrets.

const KEY_CHARACTERS = 'abcdefghijklmnopqrstuvwxyz0123456789';

/** The length of the API key of an install. The server takes no other length. */
export const API_KEY_LENGTH = 20;

/** A random key of lower-case letters and digits, each as likely as every other. */
export function randomKey(length: number = API_KEY_LENGTH): string {
  // A byte has 256 values. The values from this limit on would make the first
  // characters of the list more likely than the others, so they are thrown away.
  const limit = 256 - (256 % KEY_CHARACTERS.length);
  let key = '';
  while (key.length < length) {
    for (const byte of crypto.getRandomValues(new Uint8Array(length))) {
      if (byte < limit && key.length < length) {
        key += KEY_CHARACTERS[byte % KEY_CHARACTERS.length];
      }
    }
  }
  return key;
}
