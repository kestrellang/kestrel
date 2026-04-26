import type { RouteObject } from "react-router";
import HomePage from "./pages/HomePage";
import FlockPage from "./pages/FlockPage";
import OrgPage from "./pages/OrgPage";
import PackagePage from "./pages/PackagePage";
import StdlibIndex from "./pages/StdlibIndex";
import StdlibModule from "./pages/StdlibModule";
import StdlibItem from "./pages/StdlibItem";

export const routes: RouteObject[] = [
  { path: "/", element: <HomePage /> },
  { path: "/flock", element: <FlockPage /> },
  { path: "/flock/:org", element: <OrgPage /> },
  { path: "/flock/:org/:pkg", element: <PackagePage /> },
  { path: "/reference/stdlib", element: <StdlibIndex /> },
  { path: "/reference/stdlib/:modulePath", element: <StdlibModule /> },
  {
    path: "/reference/stdlib/:modulePath/:itemName",
    element: <StdlibItem />,
  },
];
