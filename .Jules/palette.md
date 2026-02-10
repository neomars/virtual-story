## 2025-02-07 - [Scene Editor UX & Localization]
**Learning:** Combining loading states (isSaving) with aria-busy and disabled attributes provides a robust feedback loop for both visual and screen-reader users, especially during long operations like file uploads.
**Action:** Always implement this pattern for any form that performs asynchronous operations.

## 2025-02-07 - [UI Consistency]
**Learning:** Mixed-language interfaces (e.g., English header with French form labels) significantly degrade the professional feel and predictability of an application. Localizing core UI to English while keeping narrative content in French (or its original language) is a preferred middle ground for this project.
**Action:** Proactively identify and translate remaining French UI labels in other administration views (like SceneList.vue).
