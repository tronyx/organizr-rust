// The homepage item Weather-Air: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const WEATHER_AIR: HomepageItemForm = {
  id: 'weather-air',
  name: 'Weather-Air',
  image: 'plugins/images/tabs/wind.png',
  category: 'Monitor',
  enabledKey: 'homepageWeatherAndAirEnabled',
  page: {
    id: 'homepage-weather-air',
    title: 'Weather-Air',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageWeatherAndAirEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageWeatherAndAirAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
        ],
      },
      {
        id: 'connection',
        title: 'Connection',
        entries: [
          {
            key: 'homepageWeatherAndAirLatitude',
            kind: 'text',
            label: 'Latitude',
            help: 'Please enter full latitude including minus if needed',
          },
          {
            key: 'homepageWeatherAndAirLongitude',
            kind: 'text',
            label: 'Longitude',
            help: 'Please enter full longitude including minus if needed',
          },
          {
            kind: 'button',
            label: '',
            text: 'Need Help With Coordinates?',
            action: { kind: 'open', url: 'https://www.latlong.net/' },
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'homepageWeatherAndAirWeatherHeader',
            kind: 'text',
            label: 'Title',
            help: 'Sets the title of this homepage module',
          },
          {
            key: 'homepageWeatherAndAirWeatherHeaderToggle',
            kind: 'switch',
            label: 'Toggle Title',
            help: 'Shows/hides the title of this homepage module',
          },
          {
            key: 'homepageWeatherAndAirWeatherEnabled',
            kind: 'switch',
            label: 'Enable Weather',
            help: 'Toggles the view module for Weather',
          },
          {
            key: 'homepageWeatherAndAirAirQualityEnabled',
            kind: 'switch',
            label: 'Enable Air Quality',
            help: 'Toggles the view module for Air Quality',
          },
          {
            key: 'homepageWeatherAndAirPollenEnabled',
            kind: 'switch',
            label: 'Enable Pollen',
            help: 'Toggles the view module for Pollen',
          },
          {
            key: 'homepageWeatherAndAirUnits',
            kind: 'select',
            label: 'Unit of Measurement',
            choices: [
              { label: 'Imperial', value: 'imperial' },
              { label: 'Metric', value: 'metric' },
            ],
          },
          {
            key: 'homepageWeatherAndAirRefresh',
            kind: 'select',
            label: 'Refresh Seconds',
            choices: [
              { label: '2.5', value: '2500' },
              { label: '5', value: '5000' },
              { label: '10', value: '10000' },
              { label: '15', value: '15000' },
              { label: '30', value: '30000' },
              { label: '60 [1 Minute]', value: '60000' },
              { label: '300 [5 Minutes]', value: '300000' },
              { label: '600 [10 Minutes]', value: '600000' },
              { label: '900 [15 Minutes]', value: '900000' },
              { label: '1800 [30 Minutes]', value: '1800000' },
              { label: '3600 [1 Hour]', value: '3600000' },
            ],
          },
        ],
      },
    ],
  },
};
