# Next Steps

Final playback fix is complete; roadmap stays the same and Phase 5 is still next.

## Phase 5: Docker packaging

1. Add Docker packaging for Linux audio deployment, keeping audio device access and stdout logging workable in-container.
2. Document Linux audio device access and LAN-only exposure.
3. Publish the image after repeatable container tests.

## Later optimization

1. Add a title index for constant-time title lookup.
2. Persist a monotonic id counter.
3. Benchmark decode/play latency before considering optional preloading.
