import { Routes, Route } from "react-router";
import HomePage from "./pages/HomePage";
import FlockPage from "./pages/FlockPage";
import OrgPage from "./pages/OrgPage";
import PackagePage from "./pages/PackagePage";
import StdlibIndex from "./pages/StdlibIndex";
import StdlibModule from "./pages/StdlibModule";
import StdlibItem from "./pages/StdlibItem";

function App() {
  return (
    <Routes>
      <Route path="/" element={<HomePage />} />
      <Route path="/flock" element={<FlockPage />} />
      <Route path="/flock/:org" element={<OrgPage />} />
      <Route path="/flock/:org/:pkg" element={<PackagePage />} />
      <Route path="/reference/stdlib" element={<StdlibIndex />} />
      <Route path="/reference/stdlib/:modulePath" element={<StdlibModule />} />
      <Route
        path="/reference/stdlib/:modulePath/:itemName"
        element={<StdlibItem />}
      />
    </Routes>
  );
}

export default App;
