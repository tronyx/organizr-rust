import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../../test-helpers';
import Octoprint from './Octoprint.svelte';

function show(job: unknown, webcam: unknown, titleToggle = true) {
  stubApi({
    'GET homepage/octoprint/data': {
      data: {
        data: { url: 'http://printer.example.com/', job, settings: { webcam } },
        options: { title: 'Octoprint', titleToggle },
      },
    },
  });
  render(Octoprint);
}

const PRINTING = {
  state: 'Printing',
  job: { file: { display: 'Bracket.gcode', name: 'bracket.gcode' }, estimatedPrintTime: 5400 },
  progress: { completion: 41.6, printTimeLeft: 3200 },
};

describe('Octoprint', () => {
  it('shows the print job, and the webcam of the printer', async () => {
    show(PRINTING, { webcamEnabled: true, streamUrl: '/webcam/?action=stream' });

    const job = await screen.findByLabelText('Print job');
    const text = job.textContent?.replace(/\s+/g, ' ') ?? '';
    expect(text).toContain('State Printing');
    expect(text).toContain('File Bracket.gcode');
    expect(text).toContain('42%');
    expect(text).toContain('Approx. Total Print Time 1 hour 30 minutes');
    expect(text).toContain('Print Time Left 53 minutes 20 seconds');
    // A path of the stream is one of the install.
    expect(screen.getByRole('img', { name: 'Webcam of the printer' }).getAttribute('src')).toBe(
      'http://printer.example.com/webcam/?action=stream',
    );
    expect(screen.getByRole('heading', { name: 'Octoprint' })).toBeTruthy();
  });

  it('shows only the state of a printer that does not print, and no webcam that is off', async () => {
    show(
      { state: 'Operational', job: { file: { display: null } }, progress: { completion: null } },
      { webcamEnabled: false, streamUrl: 'http://camera.example.com/stream' },
      false,
    );

    const job = await screen.findByLabelText('Print job');
    expect(job.textContent?.replace(/\s+/g, ' ')).toContain('State Operational');
    expect(job.textContent).not.toContain('File');
    expect(screen.queryByRole('img')).toBeNull();
    expect(screen.queryByRole('heading')).toBeNull();
  });

  it('shows no webcam whose address is none of the web', async () => {
    show(PRINTING, { webcamEnabled: true, streamUrl: 'javascript:alert(1)' });

    await screen.findByLabelText('Print job');
    expect(screen.queryByRole('img')).toBeNull();
  });
});
