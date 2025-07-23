<script>
  import { leftVU, rightVU } from '../stores/cassette.js';
  
  export let channel = 'left';
  
  $: level = channel === 'left' ? $leftVU : $rightVU;
  $: levelClass = level > 80 ? 'bg-red-500' : level > 60 ? 'bg-yellow-400' : 'bg-green-400';
</script>

<div class="vu-meter">
  <div class="vu-meter-bar {levelClass}" style="width: {level}%"></div>
</div>

<!-- Tick marks -->
<div class="flex justify-between mt-1 px-1">
  <span class="text-xs text-gray-600">-20</span>
  <span class="text-xs text-gray-600">-10</span>
  <span class="text-xs text-gray-600">0</span>
  <span class="text-xs text-red-600">+3</span>
</div>