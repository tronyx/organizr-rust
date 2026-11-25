import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import TagsField from './TagsField.svelte';

describe('TagsField', () => {
  it('adds typed values with Enter and a comma, without repeats', async () => {
    const onchange = vi.fn();
    render(TagsField, { id: 'ips', values: ['10.0.0.0/8'], onchange });
    const box = screen.getByRole('textbox');

    await fireEvent.input(box, { target: { value: '192.168.1.0/24' } });
    await fireEvent.keyDown(box, { key: 'Enter' });
    expect(onchange).toHaveBeenLastCalledWith(['10.0.0.0/8', '192.168.1.0/24']);

    await fireEvent.input(box, { target: { value: '10.0.0.0/8, 172.16.0.0/12' } });
    await fireEvent.keyDown(box, { key: ',' });
    expect(onchange).toHaveBeenLastCalledWith(['10.0.0.0/8', '172.16.0.0/12']);
  });

  it('removes a value with its button, and puts a clicked value back in the box', async () => {
    const onchange = vi.fn();
    render(TagsField, { id: 'urls', values: ['http://a:8181', 'http://b:8181'], onchange });

    await fireEvent.click(screen.getByRole('button', { name: 'Remove http://a:8181' }));
    expect(onchange).toHaveBeenLastCalledWith(['http://b:8181']);

    await fireEvent.click(screen.getByRole('button', { name: 'Change http://b:8181' }));
    expect(onchange).toHaveBeenLastCalledWith(['http://a:8181']);
    expect(screen.getByRole('textbox')).toHaveValue('http://b:8181');
  });

  it('names secret values by their place, not by the value', () => {
    render(TagsField, { id: 'keys', values: ['the-key'], hidden: true, onchange: vi.fn() });

    expect(screen.getByRole('button', { name: 'Remove value 1' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: /the-key/ })).toBeNull();
  });
});
