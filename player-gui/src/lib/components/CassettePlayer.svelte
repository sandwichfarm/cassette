<script>
  import { onMount } from 'svelte';
  import TapeReel from './TapeReel.svelte';
  import VUMeter from './VUMeter.svelte';
  import CassetteSlot from './CassetteSlot.svelte';
  import ControlButtons from './ControlButtons.svelte';
  import { cassetteState } from '../stores/cassette.js';
  
  let isPlaying = false;
  let isLoaded = false;
  
  $: isLoaded = $cassetteState.loaded;
  $: isPlaying = $cassetteState.playing;
</script>

<div class="cassette-player bg-cassette-black rounded-lg shadow-2xl p-8 max-w-4xl mx-auto">
  <!-- Display Screen -->
  <div class="display bg-gray-900 rounded p-4 mb-6 font-mono text-green-400 text-sm">
    <div class="flex justify-between items-center">
      <div>
        {#if isLoaded}
          <div>CASSETTE: {$cassetteState.name || 'UNKNOWN'}</div>
          <div class="text-xs opacity-75">EVENTS: {$cassetteState.eventCount || 0}</div>
        {:else}
          <div class="animate-pulse">INSERT CASSETTE</div>
        {/if}
      </div>
      <div class="text-right">
        <div>{isPlaying ? 'PLAYING' : isLoaded ? 'READY' : 'STOPPED'}</div>
        <div class="text-xs opacity-75">NIP-01 INTERFACE</div>
      </div>
    </div>
  </div>

  <!-- Tape Mechanism -->
  <div class="tape-deck bg-gray-800 rounded-lg p-6 mb-6">
    <div class="flex justify-between items-center mb-4">
      <TapeReel spinning={isPlaying} side="left" />
      <div class="flex-1 mx-8">
        <CassetteSlot />
      </div>
      <TapeReel spinning={isPlaying} side="right" />
    </div>
    
    <!-- VU Meters -->
    <div class="flex justify-center gap-4">
      <VUMeter active={isPlaying} channel="L" />
      <VUMeter active={isPlaying} channel="R" />
    </div>
  </div>

  <!-- Control Panel -->
  <ControlButtons />
</div>

<style>
  .cassette-player {
    background: linear-gradient(135deg, #1a1a1a 0%, #2d2d2d 100%);
    border: 2px solid #333;
    box-shadow: 
      inset 0 2px 4px rgba(0,0,0,0.5),
      0 4px 8px rgba(0,0,0,0.3);
  }

  .display {
    background: linear-gradient(to bottom, #0a0a0a, #1a1a1a);
    border: 2px solid #333;
    box-shadow: inset 0 2px 4px rgba(0,0,0,0.8);
  }

  .tape-deck {
    background: linear-gradient(to bottom, #2a2a2a, #1a1a1a);
    border: 1px solid #444;
    box-shadow: inset 0 2px 4px rgba(0,0,0,0.3);
  }
</style>