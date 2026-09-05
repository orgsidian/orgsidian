import "./styles/app.css";
// Story 6.7: side-effect import applies the cold-start ("system") theme to
// `document.body.dataset.theme` at module-import time, before React renders any
// content -- so no rendered UI ever paints in the wrong theme. See
// `themes/themeMode.ts` for the session-only preference store consumed by
// `AppearanceSettings` (and for the note on the residual empty-body flash).
import "./themes/themeMode";
import React from "react";
import ReactDOM from "react-dom/client";
import { RouterProvider, createRouter } from "@tanstack/react-router";
import { i18n } from "@lingui/core";
import { I18nProvider } from "@lingui/react";
import { messages as enMessages } from "./locales/en/messages";
import { routeTree } from "./routeTree.gen";

i18n.load("en", enMessages);
i18n.activate("en");

const router = createRouter({
  routeTree,
  defaultPreload: "intent",
  scrollRestoration: true,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <I18nProvider i18n={i18n}>
      <RouterProvider router={router} />
    </I18nProvider>
  </React.StrictMode>,
);
