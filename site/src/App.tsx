import { Routes, Route } from "react-router";
import HomePage from "./pages/HomePage";
import FlockPage from "./pages/FlockPage";
import OrgPage from "./pages/OrgPage";
import PackagePage from "./pages/PackagePage";

function App() {
  return (
    <Routes>
      <Route path="/" element={<HomePage />} />
      <Route path="/flock" element={<FlockPage />} />
      <Route path="/flock/:org" element={<OrgPage />} />
      <Route path="/flock/:org/:pkg" element={<PackagePage />} />
    </Routes>
  );
}

export default App;
