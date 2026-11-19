import { describe, expect, it } from 'vitest';

import { customHtmlAddress, MAX_FRAME_HEIGHT, ownStyleSheet, reportedHeight } from './homepage';

function pageWith(links: string[]): Document {
  const page = document.implementation.createHTMLDocument('test');
  const base = page.createElement('base');
  base.href = 'https://organizr.example.com/';
  page.head.append(base);
  for (const href of links) {
    const link = page.createElement('link');
    link.rel = 'stylesheet';
    link.href = href;
    page.head.append(link);
  }
  return page;
}

describe('ownStyleSheet', () => {
  it('is the path of the first style sheet of the own site', () => {
    const page = pageWith(['https://fonts.example.net/a.css', '/assets/index-abc.css']);
    expect(ownStyleSheet(page)).toBe('/assets/index-abc.css');
  });

  it('is nothing when the page has no style sheet of the own site', () => {
    expect(ownStyleSheet(pageWith([]))).toBeNull();
    expect(ownStyleSheet(pageWith(['https://fonts.example.net/a.css']))).toBeNull();
  });
});

describe('customHtmlAddress', () => {
  it('names the item and the style sheet', () => {
    expect(customHtmlAddress('01', '/assets/index-abc.css')).toMatch(
      /\/api\/v2\/homepage\/html\/01\?style=\/assets\/index-abc\.css$/,
    );
    expect(customHtmlAddress('02', null)).toMatch(/\/api\/v2\/homepage\/html\/02$/);
  });
});

describe('reportedHeight', () => {
  it('takes a height, rounded up', () => {
    expect(reportedHeight({ organizrFrameHeight: 120.2 })).toBe(121);
    expect(reportedHeight({ organizrFrameHeight: 0 })).toBe(0);
  });

  it('has a limit', () => {
    expect(reportedHeight({ organizrFrameHeight: 9e9 })).toBe(MAX_FRAME_HEIGHT);
  });

  it('takes nothing else', () => {
    for (const data of [
      null,
      'tall',
      42,
      {},
      { organizrFrameHeight: '100' },
      { organizrFrameHeight: -1 },
      { organizrFrameHeight: Number.NaN },
      { organizrFrameHeight: Number.POSITIVE_INFINITY },
    ]) {
      expect(reportedHeight(data)).toBeNull();
    }
  });
});
