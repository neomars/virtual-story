## 2025-05-14 - [Accessible custom file inputs and button styling]
**Learning:** Custom file inputs using a label-button pattern need careful styling of the label as an inline-block to ensure consistent sizing with other buttons. Using `aria-busy` and `disabled` states provides essential feedback for both visual and screen reader users during slow uploads (like videos).
**Action:** Always use `display: inline-block` and `box-sizing: border-box` when styling `router-link` or `<label>` elements as buttons, and ensure loading states are present for all async actions.
