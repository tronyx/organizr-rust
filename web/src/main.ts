import { mount } from 'svelte';

import './app.css';
import './icons/base.css';
import './icons/fontawesome.css';
import './icons/materialize.css';
import './icons/simpleline.css';
import './icons/themify.css';
import './icons/weathericon.css';
import App from './App.svelte';

const target = document.getElementById('app');
if (!target) {
  throw new Error('The page has no element with the id "app".');
}

export default mount(App, { target });
