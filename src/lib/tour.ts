// Guided tour (driver.js): dims the app and spotlights one part at a time.
import { driver, type DriveStep } from "driver.js";
import "driver.js/dist/driver.css";

const SEEN_KEY = "pw.tourSeen";

const steps: DriveStep[] = [
  {
    popover: {
      title: "Welcome to prob-warp",
      description: "Solve LeetCode problems in Java with real compiler errors and no autocomplete. This tour takes about a minute. Use ← → or the buttons.",
    },
  },
  {
    element: '[data-tour="search"]',
    popover: {
      title: "Load any problem",
      description: "Press <kbd>⌘K</kbd>, then type a name, a number, or paste a LeetCode link.",
      side: "bottom",
    },
  },
  {
    element: '[data-tour="problems"]',
    popover: {
      title: "Your problems",
      description: "They stay in the order you added them. <kbd>⌘1</kbd>–<kbd>⌘9</kbd> jumps to one. A ✓ means you've solved it.",
      side: "right",
    },
  },
  {
    element: '[data-tour="problem-tabs"]',
    popover: {
      title: "Read it, then test it",
      description: "<b>Description</b> is LeetCode's text. <b>Tests</b> holds the cases your code runs against. Edit them or add your own.",
      side: "bottom",
    },
  },
  {
    element: '[data-tour="editor"]',
    popover: {
      title: "Write Java",
      description: "Errors are underlined when you stop typing. Nothing autocompletes. Forgot how to declare something? Type <code>map</code>, <code>list</code> or <code>pq</code> then <kbd>⇥</kbd>, or press <kbd>⌘I</kbd> to search.",
      side: "left",
    },
  },
  {
    element: '[data-tour="run"]',
    popover: {
      title: "Run it",
      description: "<kbd>⌘↵</kbd> compiles and runs every test in a sandbox. <kbd>esc</kbd> stops a program that hangs.",
      side: "top",
    },
  },
  {
    element: '[data-tour="output"]',
    popover: {
      title: "See what happened",
      description: "Each run is a block: the verdict, every case's input, output and expected, and what you printed. <b>details</b> shows exactly how the program ended.",
      side: "left",
    },
  },
  {
    element: '[data-tour="tools"]',
    popover: {
      title: "When something breaks",
      description: "<b>Logs</b> shows everything the app did. <b>Settings</b> has the time and memory limits and the templates switch. Replay this tour from <b>Tour</b>.",
      side: "right",
    },
  },
];

export function startTour(onDone?: () => void) {
  const d = driver({
    steps,
    popoverClass: "pw-tour",
    showProgress: true,
    progressText: "{{current}} / {{total}}",
    nextBtnText: "Next",
    prevBtnText: "Back",
    doneBtnText: "Done",
    overlayColor: "#000",
    overlayOpacity: 0.62,
    stagePadding: 6,
    stageRadius: 10,
    smoothScroll: true,
    allowClose: true,
    disableActiveInteraction: true,
    onDestroyed: () => {
      try {
        localStorage.setItem(SEEN_KEY, "1");
      } catch {
        /* storage unavailable: the tour may show again next launch */
      }
      onDone?.();
    },
  });
  d.drive();
}

export function tourSeen(): boolean {
  try {
    return localStorage.getItem(SEEN_KEY) === "1";
  } catch {
    return true; // can't remember it, so don't nag every launch
  }
}
