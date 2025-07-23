<script>
  import { cassetteStatus, isPlaying, tapePosition } from '../stores/cassette.js';
  import TapeReel from './TapeReel.svelte';
  import VUMeter from './VUMeter.svelte';
  import ControlButtons from './ControlButtons.svelte';
  import CassetteSlot from './CassetteSlot.svelte';

  $: isLoaded = $cassetteStatus === 'loaded';
</script>

<div class="cassette-player bg-gradient-to-b from-gray-800 to-gray-900 rounded-lg p-8 shadow-2xl border-4 border-cassette-silver">
  <!-- Brand Label -->
  <div class="text-center mb-6">
    <h1 class="font-display text-3xl text-cassette-silver">CASSETTE PLAYER</h1>
    <p class="text-sm text-gray-500 mt-1">Nostr Event Replay System</p>
  </div>

  <!-- Cassette Slot -->
  <CassetteSlot />

  <!-- Tape Mechanism -->
  <div class="relative my-8">
    <div class="flex justify-between items-center px-8">
      <TapeReel spinning={$isPlaying} speed={1} />
      
      <!-- Tape Window -->
      <div class="flex-1 mx-4 relative">
        <div class="h-16 bg-black rounded-sm border-2 border-gray-700 overflow-hidden">
          <div class="h-full bg-tape-brown opacity-80 transform" 
               style="transform: translateX({$tapePosition - 50}%)">
            <div class="h-full w-full bg-gradient-to-r from-transparent via-black to-transparent opacity-20"></div>
          </div>
        </div>
        <div class="absolute inset-0 flex items-center justify-center">
          <span class="text-xs text-gray-500 font-mono">
            {#if $isPlaying}
              ▶ PLAYING
            {:else if isLoaded}
              ⏸ READY
            {:else}
              ⏹ EMPTY
            {/if}
          </span>
        </div>
      </div>
      
      <TapeReel spinning={$isPlaying} speed={-1.2} />
    </div>
  </div>

  <!-- VU Meters -->
  <div class="mb-6">
    <div class="flex gap-4 px-8">
      <div class="flex-1">
        <div class="text-xs text-gray-500 mb-1">L</div>
        <VUMeter channel="left" />
      </div>
      <div class="flex-1">
        <div class="text-xs text-gray-500 mb-1">R</div>
        <VUMeter channel="right" />
      </div>
    </div>
  </div>

  <!-- Control Buttons -->
  <ControlButtons />
</div>

<style>
  .cassette-player {
    max-width: 600px;
    background-image: 
      repeating-linear-gradient(
        0deg,
        transparent,
        transparent 2px,
        rgba(0, 0, 0, 0.1) 2px,
        rgba(0, 0, 0, 0.1) 4px
      );
  }
</style>