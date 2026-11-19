<script lang="ts">
  // The weather and air item: the weather of the next hours, the air quality and the pollen.
  import { itemData } from '../../lib/itemData.svelte';

  interface Measure {
    value?: number | null;
    units?: string | null;
  }

  interface Hour {
    datetime?: string;
    weather_text?: string;
    temperature?: Measure;
    feels_like_temperature?: Measure;
    relative_humidity?: number | null;
    precipitation?: { precipitation_probability?: number | null };
    wind?: { speed?: Measure };
  }

  interface AirIndex {
    display_name?: string;
    aqi_display?: string;
    aqi?: number;
    category?: string;
    dominant_pollutant?: string;
  }

  interface PollenKind {
    display_name?: string;
    data_available?: boolean;
    index?: { value?: number | null; category?: string | null };
  }

  interface Answer {
    content: {
      weather: false | { data?: Hour[] };
      air:
        | false
        | {
            data?: {
              indexes?: Record<string, AirIndex>;
              health_recommendations?: { general_population?: string };
            };
          };
      pollen: false | { data?: { types?: Record<string, PollenKind> }[] };
    };
  }

  /** How many of the next hours are shown. */
  const HOURS = 8;

  let { title }: { title: string } = $props();

  const item = itemData<Answer>('homepage/weather/data', 'homepageWeatherAndAirRefresh');

  const content = $derived(item.data?.content);
  const hours = $derived(content?.weather ? (content.weather.data ?? []) : []);
  const now = $derived(hours[0]);
  const air = $derived(content?.air ? content.air.data : undefined);
  const airIndex = $derived(air?.indexes?.baqi ?? Object.values(air?.indexes ?? {})[0]);
  const pollen = $derived(
    Object.values((content?.pollen ? content.pollen.data?.[0]?.types : undefined) ?? {}).filter(
      (kind) => kind.data_available !== false,
    ),
  );
  const nothing = $derived(!now && !airIndex && pollen.length === 0);

  function measure(value: Measure | undefined): string {
    if (value?.value === null || value?.value === undefined) {
      return '';
    }
    return `${Math.round(value.value)} ${value.units ?? ''}`.trim();
  }

  /** The hour of a time in the time zone of the browser: "14:00". */
  function hourOf(time: string | undefined): string {
    const moment = time ? Date.parse(time) : Number.NaN;
    if (Number.isNaN(moment)) {
      return '';
    }
    return new Intl.DateTimeFormat('en', {
      hour: '2-digit',
      minute: '2-digit',
      hour12: false,
    }).format(moment);
  }
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>Weather: {item.error}</span></div>
{:else if item.data}
  {#if title}
    <h2 class="mb-2 text-lg font-semibold">{title}</h2>
  {/if}
  {#if nothing}
    <p class="text-sm">
      The weather service gave no data. Look at the place, the API key, and the kinds of data that
      are switched on.
    </p>
  {/if}
  <div class="grid gap-2 lg:grid-cols-3">
    {#if now}
      <section class="card bg-base-200 lg:col-span-3" aria-label="Weather">
        <div class="card-body gap-2 p-3">
          <p>
            <span class="text-2xl font-semibold">{measure(now.temperature)}</span>
            <span class="ml-2">{now.weather_text ?? ''}</span>
          </p>
          <dl class="flex flex-wrap gap-x-4 text-sm">
            {#if measure(now.feels_like_temperature)}
              <div>
                <dt class="inline">Feels like</dt>
                <dd class="inline">{measure(now.feels_like_temperature)}</dd>
              </div>
            {/if}
            {#if now.relative_humidity !== null && now.relative_humidity !== undefined}
              <div>
                <dt class="inline">Humidity</dt>
                <dd class="inline">{now.relative_humidity} %</dd>
              </div>
            {/if}
            {#if measure(now.wind?.speed)}
              <div>
                <dt class="inline">Wind</dt>
                <dd class="inline">{measure(now.wind?.speed)}</dd>
              </div>
            {/if}
          </dl>
          <ul class="flex gap-3 overflow-x-auto text-sm" aria-label="The next hours">
            {#each hours.slice(1, HOURS + 1) as hour, index (index)}
              <li class="text-center">
                <div class="opacity-70">{hourOf(hour.datetime)}</div>
                <div class="font-medium">{measure(hour.temperature)}</div>
                <div class="text-xs">{hour.weather_text ?? ''}</div>
              </li>
            {/each}
          </ul>
        </div>
      </section>
    {/if}
    {#if airIndex}
      <section class="card bg-base-200" aria-label="Air quality">
        <div class="card-body gap-1 p-3">
          <h3 class="font-medium">Air quality</h3>
          <p>
            <span class="text-2xl font-semibold">{airIndex.aqi_display ?? airIndex.aqi ?? ''}</span>
            <span class="ml-2">{airIndex.category ?? ''}</span>
          </p>
          {#if air?.health_recommendations?.general_population}
            <p class="text-sm">{air.health_recommendations.general_population}</p>
          {/if}
        </div>
      </section>
    {/if}
    {#if pollen.length > 0}
      <section class="card bg-base-200" aria-label="Pollen">
        <div class="card-body gap-1 p-3">
          <h3 class="font-medium">Pollen</h3>
          <ul class="text-sm">
            {#each pollen as kind, index (index)}
              <li class="flex justify-between gap-2">
                <span>{kind.display_name ?? ''}</span>
                <span>{kind.index?.category ?? 'No data'}</span>
              </li>
            {/each}
          </ul>
        </div>
      </section>
    {/if}
  </div>
{/if}
