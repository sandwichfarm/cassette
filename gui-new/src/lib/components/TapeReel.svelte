<script>
  export let spinning = false;
  export let speed = 1;
</script>

<div class="tape-reel">
  <div class="tape-reel-inner relative {spinning ? 'animate-spin-slow' : ''}"
       style="animation-duration: {3 / Math.abs(speed)}s; animation-direction: {speed < 0 ? 'reverse' : 'normal'}">
    <!-- Reel holes -->
    <div class="absolute inset-2 flex items-center justify-center">
      <div class="w-2 h-2 bg-gray-900 rounded-full"></div>
    </div>
    <div class="absolute top-2 left-1/2 -translate-x-1/2 w-1 h-1 bg-gray-900 rounded-full"></div>
    <div class="absolute bottom-2 left-1/2 -translate-x-1/2 w-1 h-1 bg-gray-900 rounded-full"></div>
    <div class="absolute left-2 top-1/2 -translate-y-1/2 w-1 h-1 bg-gray-900 rounded-full"></div>
    <div class="absolute right-2 top-1/2 -translate-y-1/2 w-1 h-1 bg-gray-900 rounded-full"></div>
  </div>
</div>