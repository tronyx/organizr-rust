<script lang="ts">
  // The donate item: a person names an amount and pays it on a page of Stripe. The
  // item can list the donations of the person.
  import { api, apiUrl, failureText } from '../../lib/api';
  import { type DonateOptions, dollars } from '../../lib/statistics';
  import { longDateTime } from '../../lib/time';

  let { options }: { options: DonateOptions } = $props();

  interface Donation {
    date: string;
    email?: string | null;
    amount: number;
  }

  /** The amount in the field. An empty field stands for the minimum. */
  let amount = $state<number | null>(null);
  let donations = $state<Donation[] | null>(null);
  let error = $state<string | null>(null);

  /** The smallest amount, in whole dollars. */
  const least = $derived(Math.max(1, Math.ceil(options.minimum)));
  /** The amount of the payment, in whole dollars: at least the minimum. */
  const chosen = $derived(
    amount !== null && Number.isFinite(amount) ? Math.max(least, Math.floor(amount)) : least,
  );
  /** The route that starts the payment and sends the browser to Stripe. */
  const payment = $derived(apiUrl(`homepage/donate/${chosen}`));

  /** Shows the donations, or hides them. */
  async function toggleHistory() {
    if (donations) {
      donations = null;
      return;
    }
    try {
      donations = (await api<Donation[] | null>('GET', 'homepage/donate')).data ?? [];
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }
</script>

<section aria-label={options.heading || 'Donate'}>
  <div class="flex flex-wrap items-center gap-x-4">
    <h2 class="org-item-title flex-1">
      {options.heading || 'Donate'}<span aria-hidden="true">:</span>
    </h2>
    {#if options.history}
      <button type="button" class="btn btn-ghost btn-sm" onclick={toggleHistory}>
        <i class="oi oi-fontawesome fa-clock-o" aria-hidden="true"></i>
        {donations ? 'Hide my donations' : 'Show my donations'}
      </button>
    {/if}
  </div>
  {#if options.description}
    <p class="mb-2 text-sm whitespace-pre-line">{options.description}</p>
  {/if}
  <div class="join">
    <label class="input join-item">
      <span aria-hidden="true">$</span>
      <span class="sr-only">Amount in US dollars</span>
      <input type="number" min={least} step="1" placeholder={String(least)} bind:value={amount} />
    </label>
    <a class="btn btn-info join-item" href={payment} target="_blank" rel="noopener">
      Donate {dollars(chosen)}
    </a>
  </div>
  <p class="mt-1 text-xs opacity-70">
    You pay on a page of Stripe. The smallest amount is {dollars(least)}.
  </p>
  {#if error}
    <div role="alert" class="alert alert-error mt-2"><span>Donations: {error}</span></div>
  {/if}
  {#if donations}
    {#if donations.length === 0}
      <p class="mt-2 text-sm">No donations yet.</p>
    {:else}
      <div class="mt-2 overflow-x-auto">
        <table class="table-sm table" aria-label="Donations">
          <thead>
            <tr>
              <th scope="col">Date</th>
              <th scope="col">E-mail address</th>
              <th scope="col">Amount</th>
            </tr>
          </thead>
          <tbody>
            {#each donations as donation, index (index)}
              <tr>
                <td class="whitespace-nowrap">{longDateTime(donation.date)}</td>
                <td>{donation.email ?? ''}</td>
                <td>{dollars(Number(donation.amount))}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</section>
