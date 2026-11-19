import { mount } from 'svelte';

import './app.css';
import App from './App.svelte';

const target = document.getElementById('app');
if (!target) {
  throw new Error('The page has no element with the id "app".');
}

export default mount(App, { target });
