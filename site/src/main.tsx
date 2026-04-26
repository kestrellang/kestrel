import { StrictMode } from "react";
import { createRoot, hydrateRoot } from "react-dom/client";
import { createHead, UnheadProvider } from "@unhead/react/client";
import {
  createBrowserRouter,
  RouterProvider,
  type HydrationState,
} from "react-router";
import { routes } from "./routes";
import "./index.css";

const head = createHead();

const hydrationData = (
  window as Window & { __staticRouterHydrationData?: HydrationState }
).__staticRouterHydrationData;

const router = createBrowserRouter(routes, {
  ...(hydrationData ? { hydrationData } : {}),
});

const container = document.getElementById("app")!;
const tree = (
  <StrictMode>
    <UnheadProvider head={head}>
      <RouterProvider router={router} />
    </UnheadProvider>
  </StrictMode>
);

// Hydrate only when prerendered HTML is present (SSG build).
// In `vite dev` the container is empty, so use createRoot to avoid mismatch.
if (hydrationData) {
  hydrateRoot(container, tree);
} else {
  createRoot(container).render(tree);
}
