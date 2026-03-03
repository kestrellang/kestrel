import CodeDemo from "../components/CodeDemo";
import Features from "../components/Features";
import Footer from "../components/Footer";
import GetStarted from "../components/GetStarted";
import Hero from "../components/Hero";

export default function HomePage() {
  return (
    <div className="scroll-container">
      <Hero />
      <Features />
      <CodeDemo />
      {/* Last section: GetStarted + Footer together for natural scrolling */}
      <div className="scroll-section min-h-screen flex flex-col">
        <GetStarted />
        <Footer />
      </div>
    </div>
  );
}
