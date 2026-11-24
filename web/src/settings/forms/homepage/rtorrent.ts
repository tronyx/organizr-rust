// The homepage item rTorrent: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const RTORRENT: HomepageItemForm = {
  id: 'rtorrent',
  name: 'rTorrent',
  image: 'plugins/images/tabs/rTorrent.png',
  category: 'Downloader',
  enabledKey: 'homepagerTorrentEnabled',
  license: 'personal',
  page: {
    id: 'homepage-rtorrent',
    title: 'rTorrent',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepagerTorrentEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepagerTorrentAuth',
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
            key: 'rTorrentURL',
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          {
            key: 'rTorrentURLOverride',
            kind: 'text',
            label: 'rTorrent API URL Override',
            help: 'Only use if you cannot connect. Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port/xmlrpc',
          },
          { key: 'rTorrentUsername', kind: 'text', label: 'Username' },
          { key: 'rTorrentPassword', kind: 'secret', label: 'Password' },
          { key: 'rTorrentDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'rTorrentUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/rtorrent', method: 'POST' },
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          { key: 'rTorrentHideSeeding', kind: 'switch', label: 'Hide Seeding' },
          { key: 'rTorrentHideCompleted', kind: 'switch', label: 'Hide Completed' },
          {
            key: 'rTorrentSortOrder',
            kind: 'select',
            label: 'Order',
            choices: [
              { label: 'Date Desc', value: 'dated' },
              { label: 'Date Asc', value: 'datea' },
              { label: 'Hash Desc', value: 'hashd' },
              { label: 'Hash Asc', value: 'hasha' },
              { label: 'Name Desc', value: 'named' },
              { label: 'Name Asc', value: 'namea' },
              { label: 'Size Desc', value: 'sized' },
              { label: 'Size Asc', value: 'sizea' },
              { label: 'Label Desc', value: 'labeld' },
              { label: 'Label Asc', value: 'labela' },
              { label: 'Status Desc', value: 'statusd' },
              { label: 'Status Asc', value: 'statusa' },
            ],
          },
          { key: 'rTorrentLimit', kind: 'number', label: 'Item Limit' },
          { key: 'rTorrentIgnoreLabel', kind: 'tags', label: 'Ignore Torrent with Label(s)' },
          {
            key: 'rTorrentRefresh',
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
          { key: 'rTorrentCombine', kind: 'switch', label: 'Add to Combined Downloader' },
        ],
      },
    ],
  },
};
