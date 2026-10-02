import { Header } from "../components/Header";
import { PlayButton } from "../components/PlayButton";
import { ServerStatus } from "../components/ServerStatus";

export function Home() {
  return (
    <div className="launcher">
      <Header />

      <main className="home">
        <div className="hero">
          <p className="small-title">BIENVENUE SUR</p>

          <h1>ETERNIA</h1>

          <p className="subtitle">
            Une nouvelle aventure commence.
          </p>

          <PlayButton />

          <ServerStatus />
        </div>
      </main>
    </div>
  );
}