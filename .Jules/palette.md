## 2025-05-15 - [Ajout d'un bouton de suppression des scènes]
**Learning:** Adding a critical action like deletion requires clear visual differentiation (color) and a confirmation dialog to prevent user errors.
**Action:** Always use color-coding (red for danger/delete) and `window.confirm` (or a modal) for irreversible actions in admin interfaces.

## 2025-05-16 - [Amélioration UX et Accessibilité du Lecteur]
**Learning:** Using `opacity: 0.3` for interactive elements like choice links severely impacts accessibility and readability. Transitions between states (e.g., video end to choices list) provide a much smoother user experience than abrupt content changes.
**Action:** Use full opacity for primary interactive elements and implement `:focus-visible` for keyboard navigation. Use Vue's `<Transition>` component for state-dependent UI elements.

## 2025-05-17 - [Autoplay et Contraintes Navigateur]
**Learning:** Browsers block unmuted autoplay and automatic fullscreen requests without user interaction.
**Action:** Always wrap `.play()` in a try-catch, use `playsinline` for mobile, and separate automatic playback from manual playback to handle fullscreen gracefully.

## 2025-05-18 - [Expérience Vidéo "Pleine Page" sans Fullscreen API]
**Learning:** For a focused video experience, using CSS `position: fixed` to cover the viewport is more reliable and less intrusive than the browser's native Fullscreen API, especially when you need to transition back to standard UI elements like choices.
**Action:** Use dynamic CSS classes to toggle a "full-page" state during video playback, covering the UI until the video ends.

## 2025-05-19 - [Outil de Synchronisation de Base de Données en Admin]
**Learning:** When deploying schema changes to environments where migrations might fail or be manual, providing a dedicated "Sync" button in the Admin UI with clear instruction text helps users resolve errors autonomously without needing direct DB access.
**Action:** Implement "Idempotent Schema Sync" endpoints and UI buttons when adding new database-backed features to legacy or self-hosted applications.

## 2025-05-20 - [Amélioration de l'Accessibilité et du Feedback en Admin]
**Learning:** Icon-only buttons (like "x" for delete) are invisible to screen readers without aria-label. Providing immediate feedback by disabling buttons during async operations prevents double-submissions and improves perceived reliability.
**Action:** Always add aria-label to icon-only buttons and implement a loading or isSaving state to disable interactive elements during background tasks.
