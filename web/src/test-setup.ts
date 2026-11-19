// Runs before each test file: adds the checks for elements of a page, and removes the
// components of a test from the page afterwards.
import '@testing-library/jest-dom/vitest';

import { cleanup } from '@testing-library/svelte';
import { afterEach } from 'vitest';

// jsdom, which stands in for the browser in these tests, cannot open a dialog. These
// two functions do the part that the components rely on.
HTMLDialogElement.prototype.showModal ??= function showModal(this: HTMLDialogElement) {
  this.setAttribute('open', '');
};
HTMLDialogElement.prototype.close ??= function close(this: HTMLDialogElement) {
  this.removeAttribute('open');
  this.dispatchEvent(new Event('close'));
};

afterEach(() => {
  cleanup();
});
