export const DEVICE_OPTIONS = [
  { label: 'Mouse', value: 'Mouse' },
  { label: 'Keyboard', value: 'Keyboard' },
]

export const BUTTON_OPTIONS = [
  { label: 'Left', value: 'Left' },
  { label: 'Right', value: 'Right' },
  { label: 'Middle', value: 'Middle' },
]

export const CLICK_TYPE_OPTIONS = [
  { label: 'Single', value: 'Single' },
  { label: 'Double', value: 'Double' },
  { label: 'Randomize', value: 'Randomized' },
  { label: 'Hold', value: 'Hold' },
]

/** The same click types as `CLICK_TYPE_OPTIONS`, worded for the basic home page.
 * The values are identical: both selects write to the one target on the backend. */
export const HOME_CLICK_TYPE_OPTIONS = [
  { label: 'Click', value: 'Single' },
  { label: 'Double Click', value: 'Double' },
  { label: 'Random', value: 'Randomized' },
  { label: 'Hold', value: 'Hold' },
]

/** Shown under the click type while Hold is selected: the input stays down
 * between clicks, and a CPS of zero means it never comes up at all. */
export const HOLD_HINT =
  'Hold keeps the input pressed between clicks. Set CPS to 0 to hold it down until you stop the clicker.'

/** Shown when CPS is 0 while Hold is off, where zero falls back to the one click
 * per second the scheduler has always used for a non-positive CPS. */
export const ZERO_CPS_HINT =
  'CPS is 0 and Hold is off, so this clicks once per second. Pick Hold to hold the input down instead.'

/** Shown under the hold time of a sequence step, which holds for its own wait
 * instead of for a share of the global CPS. */
export const SEQUENCE_HOLD_HINT =
  'This step holds its input down for the hold time, then releases it briefly so the next hold registers. A hold time of 0 clicks without holding.'

/** Mirrors `MIN_CPS` / `MAX_CPS` in `src-tauri/src/frontend_api.rs`, which
 * re-validates every value before it reaches the scheduler. */
export const CPS_MIN = 0
export const CPS_MAX = 1000

export const MODE_NORMAL = 0
export const MODE_SEQUENCE = 1
