import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../../test-helpers';
import Weather from './Weather.svelte';

function hour(time: string, text: string, degrees: number) {
  return {
    datetime: time,
    weather_text: text,
    temperature: { value: degrees, units: 'C' },
    feels_like_temperature: { value: degrees - 1, units: 'C' },
    relative_humidity: 61,
    wind: { speed: { value: 12.4, units: 'km/h' } },
  };
}

const WEATHER = {
  data: [
    hour('2020-01-05T12:00:00Z', 'Partly cloudy', 7.5),
    hour('2020-01-05T13:00:00Z', 'Cloudy', 8),
  ],
};
const AIR = {
  data: {
    indexes: { baqi: { aqi_display: '67', category: 'Good air quality' } },
    health_recommendations: { general_population: 'You have no limitations.' },
  },
};
const POLLEN = {
  data: [
    {
      types: {
        grass: { display_name: 'Grass', data_available: true, index: { category: 'Very Low' } },
        weed: { display_name: 'Weed', data_available: false, index: { category: null } },
      },
    },
  ],
};

describe('Weather', () => {
  it('shows the weather of this hour and of the next ones, the air and the pollen', async () => {
    stubApi({
      'GET homepage/weather/data': {
        data: { content: { weather: WEATHER, air: AIR, pollen: POLLEN } },
      },
    });
    render(Weather, { title: 'At home' });

    const weather = await screen.findByRole('region', { name: 'Weather' });
    expect(weather.textContent).toContain('8 C');
    expect(weather.textContent).toContain('Partly cloudy');
    expect(weather.textContent).toContain('Feels like 7 C');
    expect(weather.textContent).toContain('Humidity 61 %');
    expect(weather.textContent).toContain('Wind 12 km/h');
    const next = within(weather).getByRole('list', { name: 'The next hours' });
    expect(within(next).getAllByRole('listitem')).toHaveLength(1);
    expect(next.textContent).toContain('Cloudy');

    const air = screen.getByRole('region', { name: 'Air quality' });
    expect(air.textContent).toContain('67');
    expect(air.textContent).toContain('Good air quality');
    expect(air.textContent).toContain('You have no limitations.');

    // A kind of pollen without data is left out.
    const pollen = screen.getByRole('region', { name: 'Pollen' });
    expect(pollen.textContent).toContain('Grass');
    expect(pollen.textContent).toContain('Very Low');
    expect(pollen.textContent).not.toContain('Weed');
    expect(screen.getByRole('heading', { name: 'At home' })).toBeTruthy();
  });

  it('shows only the kinds that the server gave', async () => {
    stubApi({
      'GET homepage/weather/data': {
        data: { content: { weather: false, air: AIR, pollen: false } },
      },
    });
    render(Weather, { title: '' });

    expect(await screen.findByRole('region', { name: 'Air quality' })).toBeTruthy();
    expect(screen.queryByRole('region', { name: 'Weather' })).toBeNull();
    expect(screen.queryByRole('region', { name: 'Pollen' })).toBeNull();
    expect(screen.queryByRole('heading', { level: 2 })).toBeNull();
  });

  it('says so when the service gave nothing', async () => {
    stubApi({
      'GET homepage/weather/data': {
        data: { content: { weather: false, air: false, pollen: false } },
      },
    });
    render(Weather, { title: '' });

    expect(await screen.findByText(/The weather service gave no data/)).toBeTruthy();
  });
});
