import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { at, stubApi } from '../../test-helpers';
import Netdata from './Netdata.svelte';

const SHOWN = { lg: true, md: true, sm: true };

describe('Netdata', () => {
  it('shows a ring or a gauge for each chart, with its number', async () => {
    stubApi({
      'GET homepage/netdata/data': {
        data: {
          url: '',
          data: [
            {
              title: 'CPU',
              value: 12.54,
              percent: 12.54,
              units: '%',
              max: 100,
              chart: 'easypiechart',
              colour: '46e302',
              size: 'lg',
              ...SHOWN,
            },
            {
              title: 'Disk write',
              value: 3,
              percent: 250,
              units: 'MiB/s',
              max: 6000,
              chart: 'gauge',
              colour: 'not a color',
              size: '',
              lg: true,
              md: false,
              sm: false,
            },
            { error: 'custom definition not found', title: 'Custom', chart: 'gauge', ...SHOWN },
            { title: 'Swap', chart: '', colour: '', size: 'sm', ...SHOWN },
          ],
        },
      },
    });
    const { container } = render(Netdata);

    const cpu = await screen.findByRole('figure', { name: 'CPU: 12.5 %' });
    expect(cpu.style.width).toBe('180px');
    expect(cpu.className).toContain('lg:inline-block');
    expect(at(cpu.querySelectorAll('circle'), 1).getAttribute('stroke')).toBe('#46e302');

    const disk = screen.getByRole('figure', { name: 'Disk write: 3.0 MiB/s' });
    expect(disk.style.width).toBe('250px');
    // The chart shows on large screens only.
    expect(disk.className).toContain('hidden');
    expect(disk.className).toContain('md:hidden');
    expect(disk.className).toContain('lg:inline-block');
    // A color that is none is the color of the theme, and the part is 100 at most.
    const arc = at(disk.querySelectorAll('path'), 1);
    expect(arc.getAttribute('stroke')).toBe('var(--color-info)');
    expect(arc.getAttribute('stroke-dasharray')).toBe(`${Math.PI * 40} ${Math.PI * 40}`);

    expect(screen.getByText('Chart 3: custom definition not found')).toBeTruthy();
    // A chart without a value, for example when Netdata gave no answer.
    expect(container.querySelectorAll('figure')).toHaveLength(3);
    const swap = at(container.querySelectorAll('figure'), 2);
    expect(swap.getAttribute('aria-label')).toContain('Swap: –');
    expect(swap.style.width).toBe('140px');
  });

  it('says so when no chart is on', async () => {
    stubApi({ 'GET homepage/netdata/data': { data: { url: '', data: [] } } });
    render(Netdata);

    expect(await screen.findByText(/Netdata has no chart/)).toBeTruthy();
  });
});
