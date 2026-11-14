# The start-up measurement

Left over from Milestone 0, the measuring spike that decided whether to port at all (sizes, start-up,
WebKitGTK, keys, text widths). What remains is the measurement of start-up times, used for every
release; the results are in the root README, [Measurements](../../README.md#measurements).

```sh
python3 spike/measure_startup.py --runs 5 --record 3.5 -- COMMAND FILE   # any app, by screenshots
ERDTEX_MEASURE_STARTUP=1 target/release/erdtex                        # prints ready_ms= once the diagram is painted
```

`measure_startup.py` starts the program on an invisible display (Xvfb) with a fresh settings folder,
takes screenshots, and reports the first moment the screen is within 0.5 % of its final state, with the
median of the runs. It pauses 8 s between runs (a WebView started again at once waits for the last one).
`--record 3.5` stops before the status message disappears (after 4 s), which would count otherwise.
Version 2 is measured the same way (`-- desktop_app/run.sh FILE`, or its AppImage).
