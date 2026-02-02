## 2025-05-15 - Improving the Interactive Player UX

**Learning:** Low-opacity links (e.g., `opacity: 0.3`) are a common accessibility barrier, causing insufficient color contrast. Using `<Transition mode="out-in">` with unique keys is essential for swapping UI elements smoothly without layout shifts.

**Action:** Always check for `opacity` on text elements during UX audits. Use the Vue `mode="out-in"` pattern for element swaps. Ensure focus indicators are visually distinct (e.g., using `box-shadow` and `outline: transparent`).
