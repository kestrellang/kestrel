// Linked in with --js-library. Imports the game calls from C.

addToLibrary({
  // Pause the game until the browser's next animation frame, then resume
  // it inside that frame's callback. Waiting on requestAnimationFrame
  // instead of a timer keeps frames in step with the display, so motion
  // is smooth and the frame rate tops out at the screen's refresh rate.
  aerie_wait_frame__async: true,
  aerie_wait_frame: function() {
    return Asyncify.handleSleep((wakeUp) => requestAnimationFrame(() => wakeUp()));
  },
});
