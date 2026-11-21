import { mount } from 'svelte';

import '@fontsource-variable/rubik/wght.css';
import './app.css';
import './icons/base.css';
import './icons/fontawesome.css';
import './icons/materialize.css';
import './icons/simpleline.css';
import './icons/themify.css';
import './icons/weathericon.css';
import App from './App.svelte';
import { startLanguages } from './lib/i18n.svelte';

const target = document.getElementById('app');
if (!target) {
  throw new Error('The page has no element with the id "app".');
}

const app = mount(App, { target });
void startLanguages();

export default app;
