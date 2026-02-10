## 2025-05-15 - [Keyboard Shortcuts for Narrative Choices]
**Learning:** Adding numerical keyboard shortcuts (1-9) to branching narrative choices significantly improves the "flow" of interactive stories, making them feel more like an application than just a video player. Visual hints like `[n]` are essential for discoverability.
**Action:** Always consider keyboard ergonomics for narrative-heavy applications, especially those requiring frequent user decisions.

## 2025-05-15 - [Accessibility in Loading/Error States]
**Learning:** Users with screen readers benefit greatly from `aria-live="polite"` on loading indicators and `role="alert"` on error messages, ensuring they are informed of state changes without being interrupted by aggressive announcements.
**Action:** Consistently apply these attributes to all asynchronous UI states.
