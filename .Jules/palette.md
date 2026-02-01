## 2025-05-14 - [Accessible custom file inputs and button styling]
**Learning:** Custom file inputs using a label-button pattern need careful styling of the label as an inline-block to ensure consistent sizing with other buttons. Using `aria-busy` and `disabled` states provides essential feedback for both visual and screen reader users during slow uploads (like videos).
**Action:** Always use `display: inline-block` and `box-sizing: border-box` when styling `router-link` or `<label>` elements as buttons, and ensure loading states are present for all async actions.

## 2025-05-15 - [Support for High Contrast Mode in focus states]
**Learning:** Using `outline: none` to remove the default focus ring for custom `box-shadow` rings can make interactive elements invisible in Windows High Contrast Mode. Using `outline: 2px solid transparent` ensures the focus ring is visible to users in forced-colors modes while remaining hidden in standard modes.
**Action:** Always use a transparent outline when replacing the default focus ring with a custom design.
