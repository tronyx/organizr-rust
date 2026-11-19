// Runs before each test file: adds the checks for elements of a page, and removes the
// components of a test from the page afterwards.
import '@testing-library/jest-dom/vitest';

import { cleanup } from '@testing-library/svelte';
import { afterEach } from 'vitest';

afterEach(() => {
  cleanup();
});
