// The homepage item qBittorrent: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const QBITTORRENT: HomepageItemForm = {
  id: 'qbittorrent',
  name: 'qBittorrent',
  image: 'plugins/images/tabs/qBittorrent.png',
  category: 'Downloader',
  enabledKey: 'homepageqBittorrentEnabled',
  license: 'personal',
  page: {
    id: 'homepage-qbittorrent',
    title: 'qBittorrent',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageqBittorrentEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageqBittorrentAuth',
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
            key: 'qBittorrentURL',
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          {
            key: 'qBittorrentApiVersion',
            kind: 'select',
            label: 'API Version',
            choices: [
              { label: 'V1', value: '1' },
              { label: 'V2', value: '2' },
            ],
          },
          { key: 'qBittorrentUsername', kind: 'text', label: 'Username' },
          { key: 'qBittorrentPassword', kind: 'secret', label: 'Password' },
          {
            key: 'qBittorrentDisableCertCheck',
            kind: 'switch',
            label: 'Disable Certificate Check',
          },
          {
            key: 'qBittorrentUseCustomCertificate',
            kind: 'switch',
            label: 'Use Custom Certificate',
          },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/qbittorrent', method: 'POST' },
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          { key: 'qBittorrentHideSeeding', kind: 'switch', label: 'Hide Seeding' },
          { key: 'qBittorrentHideCompleted', kind: 'switch', label: 'Hide Completed' },
          {
            key: 'qBittorrentSortOrder',
            kind: 'select',
            label: 'Order',
            choices: [
              { label: 'Hash', value: 'hash' },
              { label: 'Name', value: 'name' },
              { label: 'Size', value: 'size' },
              { label: 'Progress', value: 'progress' },
              { label: 'Download Speed', value: 'dlspeed' },
              { label: 'Upload Speed', value: 'upspeed' },
              { label: 'Priority', value: 'priority' },
              { label: 'Number of Seeds', value: 'num_seeds' },
              { label: 'Number of Seeds in Swarm', value: 'num_complete' },
              { label: 'Number of Leechers', value: 'num_leechs' },
              { label: 'Number of Leechers in Swarm', value: 'num_incomplete' },
              { label: 'Ratio', value: 'ratio' },
              { label: 'ETA', value: 'eta' },
              { label: 'State', value: 'state' },
              { label: 'Category', value: 'category' },
            ],
          },
          { key: 'qBittorrentReverseSorting', kind: 'switch', label: 'Reverse Sorting' },
          {
            key: 'qBittorrentRefresh',
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
          { key: 'qBittorrentCombine', kind: 'switch', label: 'Add to Combined Downloader' },
        ],
      },
      {
        id: 'socks',
        title: 'API SOCKS',
        entries: [
          { kind: 'socks', app: 'qbittorrent', wide: true },
          { key: 'qBittorrentSocksEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'qBittorrentSocksAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
        ],
      },
    ],
  },
};
