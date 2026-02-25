# Palette's UX Journal

## 2025-05-14 - Keyboard Shortcuts for Narrative Choices
**Learning:** Power users of interactive video applications appreciate quick navigation. Providing numerical shortcuts (1-9) for narrative choices allows for a more fluid "lean-forward" experience without needing to reach for the mouse.
**Action:** Always implement numerical keyboard shortcuts when presenting a limited list of choices in an interactive experience.

## 2025-05-14 - Icon-only Button Accessibility
**Learning:** Buttons using only icons (like '×' for delete) are inaccessible to screen reader users if they lack an explicit ARIA label. The symbol is often read as "times" or "multiplication sign" instead of its intended action.
**Action:** Ensure all icon-only buttons have an `aria-label` that describes the action in the context of the specific object being manipulated (e.g., "Delete chapter" instead of just "Delete").

## 2025-05-15 - Visual Feedback for Background Operations
**Learning:** For user actions that trigger background API calls (like drag-and-drop reordering or form submissions), providing immediate and transient visual status indicators (e.g., "Saving...", "Saved!") significantly improves the perceived responsiveness of the application and reduces user uncertainty.
**Action:** Implement transient status badges or loading states for all administrative operations that don't result in immediate page navigation.

## 2025-05-20 - Contextual ARIA Labels for Narrative Graphs
**Learning:** In complex narrative editors, generic labels like "Delete choice" or "Remove link" are confusing for screen reader users when multiple links exist. Including the target scene's title in the `aria-label` provides the necessary context to make destructive actions safe and understandable.
**Action:** Use dynamic attributes to include object titles in accessibility labels for repetitive action buttons.

## 2025-05-20 - Accessibility of Keyboard Shortcuts
**Learning:** Visual-only keyboard shortcut hints (like `[1]`) that are `aria-hidden` are helpful for sighted users but leave screen reader users unaware of the functionality. Including the shortcut information in the element's `aria-label` (e.g., "Choice 1: [Text]") ensures parity of experience.
**Action:** Always include keyboard shortcut triggers in the accessible descriptions of interactive elements when those shortcuts are a primary means of interaction.

## 2025-05-22 - Visual Feedback for Secondary Admin Actions
**Learning:** In complex forms with multiple secondary actions (like linking scenes in a side panel), users benefit from immediate feedback that their click was registered. Using a ".mini" button variant with a loading state (e.g., "Adding...") prevents accidental multiple submissions and makes the interface feel more responsive.
**Action:** Implement loading states and disabled visual styles for all secondary submission buttons in the admin interface.

## 2025-05-24 - Narrative Continuity and Player Utility
**Learning:** In interactive video experiences, users often want to skip familiar content or replay a significant scene without navigating away. Providing explicit "Skip" and "Replay" utilities directly in the player interface, accompanied by keyboard shortcuts, significantly enhances narrative flow and accessibility.
**Action:** Always provide "Skip" and "Replay" controls in the primary player UI, and ensure "Restart" options are presented when a narrative path reaches its conclusion.
