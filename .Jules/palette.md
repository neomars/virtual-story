# Palette's UX Journal

## 2025-05-14 - Keyboard Shortcuts for Narrative Choices
**Learning:** Power users of interactive video applications appreciate quick navigation. Providing numerical shortcuts (1-9) for narrative choices allows for a more fluid "lean-forward" experience without needing to reach for the mouse.
**Action:** Always implement numerical keyboard shortcuts when presenting a limited list of choices in an interactive experience.

## 2025-05-14 - Icon-only Button Accessibility
**Learning:** Buttons using only icons (like '×' for delete) are inaccessible to screen reader users if they lack an explicit ARIA label. The symbol is often read as "times" or "multiplication sign" instead of its intended action.
**Action:** Ensure all icon-only buttons have an `aria-label` that describes the action in the context of the specific object being manipulated (e.g., "Delete chapter" instead of just "Delete").
