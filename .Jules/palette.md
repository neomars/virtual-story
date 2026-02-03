## 2025-05-15 - [Ajout d'un bouton de suppression des scènes]
**Learning:** Adding a critical action like deletion requires clear visual differentiation (color) and a confirmation dialog to prevent user errors.
**Action:** Always use color-coding (red for danger/delete) and `window.confirm` (or a modal) for irreversible actions in admin interfaces.

## 2025-05-16 - [Amélioration UX et Accessibilité du Lecteur]
**Learning:** Using `opacity: 0.3` for interactive elements like choice links severely impacts accessibility and readability. Transitions between states (e.g., video end to choices list) provide a much smoother user experience than abrupt content changes.
**Action:** Use full opacity for primary interactive elements and implement `:focus-visible` for keyboard navigation. Use Vue's `<Transition>` component for state-dependent UI elements.

## 2025-05-17 - [Autoplay et Contraintes Navigateur]
**Learning:** Browsers block unmuted autoplay and automatic fullscreen requests without user interaction.
**Action:** Always wrap `.play()` in a try-catch, use `playsinline` for mobile, and separate automatic playback from manual playback to handle fullscreen gracefully.
