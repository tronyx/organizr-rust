// What the interface knows about the install and the caller: the answer of `GET /launch`.

import { api, failureText } from './api';
import { CO_ADMIN, type Launch, type MenuCategory, type MenuTab } from './types';

class Session {
  /** The last answer of `GET /launch`. `null` until the first one arrives. */
  launch = $state<Launch | null>(null);
  /** Why the last load failed. */
  error = $state<string | null>(null);
  loading = $state(false);

  /** Whether the install waits for the set-up wizard. */
  needsSetUp = $derived(this.launch?.status.status === 'wizard');
  loggedIn = $derived(this.launch?.user?.loggedin === true);
  /** Whether the caller may manage tabs and categories. */
  canManage = $derived(this.loggedIn && (this.launch?.user?.groupID ?? 999) <= CO_ADMIN);
  tabs = $derived<MenuTab[]>(this.launch?.tabs || []);
  categories = $derived<MenuCategory[]>(this.launch?.categories || []);

  /** Asks the server again. Call it at the start and after a login, a logout or a change. */
  async load(): Promise<void> {
    this.loading = true;
    try {
      const { data } = await api<Launch>('GET', 'launch');
      this.launch = data;
      this.error = null;
      document.title = data.appearance.title || 'Organizr';
    } catch (failure) {
      this.error = failureText(failure);
    } finally {
      this.loading = false;
    }
  }
}

export const session = new Session();
