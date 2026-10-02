import { emit, listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useRef, useState } from "react";

import "./Console.css";

type ConsoleKind = "normal" | "info" | "error";

interface ConsoleLine {
  kind: ConsoleKind;
  message: string;
}

interface LauncherConsoleEvent {
  stream: "stdout" | "stderr";
  message: string;
}

interface LauncherProgressEvent {
  step: number;
  total: number;
  message: string;
}

interface LauncherDownloadEvent {
  current: number;
  total: number;
  percentage: number;
  message: string;
}

interface MinecraftStartedEvent {
  pid: number;
}

interface MinecraftStoppedEvent {
  pid: number;
  exit_code: number | null;
  success: boolean;
}

function Console() {
  console.log("🔥 CONSOLE.TSX CHARGÉE");

  const [lines, setLines] = useState<ConsoleLine[]>([]);
  const [search, setSearch] = useState("");
  const [autoScroll, setAutoScroll] = useState(true);
  const [paused, setPaused] = useState(false);

  const [minecraftRunning, setMinecraftRunning] =
    useState(false);

  const [minecraftPid, setMinecraftPid] =
    useState<number | null>(null);

  const [minecraftStartedAt, setMinecraftStartedAt] =
    useState<number | null>(null);

  const [uptime, setUptime] = useState(0);

  const endRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const addLine = (
      kind: ConsoleKind,
      message: string,
    ) => {
      setLines((current) => [
        ...current,
        {
          kind,
          message,
        },
      ]);
    };

    // --------------------------------------------------
    // CONSOLE MINECRAFT
    // --------------------------------------------------

    const unlistenConsole = listen<LauncherConsoleEvent>(
      "launcher-console",
      (event) => {
        const isError =
          event.payload.stream === "stderr";

        addLine(
          isError ? "error" : "normal",
          event.payload.message,
        );
      },
    );

    // --------------------------------------------------
    // PROGRESSION DU LAUNCHER
    // --------------------------------------------------

    const unlistenProgress =
      listen<LauncherProgressEvent>(
        "launcher-progress",
        (event) => {
          addLine(
            "info",
            `[${event.payload.step}/${event.payload.total}] ${event.payload.message}`,
          );
        },
      );

    // --------------------------------------------------
    // TÉLÉCHARGEMENTS
    // --------------------------------------------------

    const unlistenDownload =
      listen<LauncherDownloadEvent>(
        "launcher-download-progress",
        (event) => {
          addLine(
            "info",
            `${event.payload.message} (${event.payload.percentage}%)`,
          );
        },
      );

    // --------------------------------------------------
    // MINECRAFT DÉMARRÉ
    // --------------------------------------------------

    const unlistenStarted =
      listen<MinecraftStartedEvent>(
        "minecraft-started",
        (event) => {
          const pid = event.payload.pid;

          setMinecraftRunning(true);
          setMinecraftPid(pid);
          setMinecraftStartedAt(Date.now());
          setUptime(0);

          addLine(
            "info",
            `Minecraft démarré. PID : ${pid}`,
          );
        },
      );

    // --------------------------------------------------
    // MINECRAFT ARRÊTÉ
    // --------------------------------------------------

    const unlistenStopped =
      listen<MinecraftStoppedEvent>(
        "minecraft-stopped",
        (event) => {
          const {
            pid,
            exit_code,
            success,
          } = event.payload;

          setMinecraftRunning(false);
          setMinecraftPid(null);
          setMinecraftStartedAt(null);
          setUptime(0);

          const code =
            exit_code === null
              ? "aucun code"
              : `code ${exit_code}`;

          addLine(
            success ? "info" : "error",
            `Minecraft arrêté. PID : ${pid} | ${code}`,
          );
        },
      );

    // --------------------------------------------------
    // EFFACEMENT DE LA CONSOLE
    // --------------------------------------------------

    const unlistenClear = listen(
      "launcher-console-clear",
      () => {
        setLines([]);
      },
    );

    // --------------------------------------------------
    // CONSOLE PRÊTE
    // --------------------------------------------------

    void emit("launcher-console-ready");

    return () => {
      unlistenConsole.then((fn) => fn());
      unlistenProgress.then((fn) => fn());
      unlistenDownload.then((fn) => fn());
      unlistenStarted.then((fn) => fn());
      unlistenStopped.then((fn) => fn());
      unlistenClear.then((fn) => fn());
    };
  }, []);

  // --------------------------------------------------
  // CHRONOMÈTRE MINECRAFT
  // --------------------------------------------------

  useEffect(() => {
    if (!minecraftRunning || minecraftStartedAt === null) {
      return;
    }

    const updateUptime = () => {
      const elapsed = Math.floor(
        (Date.now() - minecraftStartedAt) / 1000,
      );

      setUptime(elapsed);
    };

    updateUptime();

    const interval = window.setInterval(
      updateUptime,
      1000,
    );

    return () => {
      window.clearInterval(interval);
    };
  }, [
    minecraftRunning,
    minecraftStartedAt,
  ]);

  // --------------------------------------------------
  // AUTO-SCROLL
  // --------------------------------------------------

  useEffect(() => {
    if (!autoScroll || paused) {
      return;
    }

    endRef.current?.scrollIntoView({
      behavior: "smooth",
    });
  }, [
    lines,
    autoScroll,
    paused,
  ]);

  // --------------------------------------------------
  // RECHERCHE
  // --------------------------------------------------

  const filteredLines = useMemo(() => {
    const query = search.trim().toLowerCase();

    if (!query) {
      return lines;
    }

    return lines.filter((line) =>
      line.message
        .toLowerCase()
        .includes(query),
    );
  }, [lines, search]);

  // --------------------------------------------------
  // FORMATAGE DU TEMPS
  // --------------------------------------------------

  const formatUptime = (
    totalSeconds: number,
  ) => {
    const hours = Math.floor(
      totalSeconds / 3600,
    );

    const minutes = Math.floor(
      (totalSeconds % 3600) / 60,
    );

    const seconds =
      totalSeconds % 60;

    if (hours > 0) {
      return `${String(hours).padStart(
        2,
        "0",
      )}:${String(minutes).padStart(
        2,
        "0",
      )}:${String(seconds).padStart(
        2,
        "0",
      )}`;
    }

    return `${String(minutes).padStart(
      2,
      "0",
    )}:${String(seconds).padStart(
      2,
      "0",
    )}`;
  };

  // --------------------------------------------------
  // VIDER
  // --------------------------------------------------

  const clearConsole = () => {
    setLines([]);
  };

  // --------------------------------------------------
  // COPIER
  // --------------------------------------------------

  const copyConsole = async () => {
    const text = lines
      .map((line) => {
        const prefix =
          line.kind === "error"
            ? "[ERR]"
            : line.kind === "info"
              ? "[INFO]"
              : "[MC]";

        return `${prefix} ${line.message}`;
      })
      .join("\n");

    try {
      await navigator.clipboard.writeText(
        text,
      );
    } catch (error) {
      console.error(
        "Impossible de copier la console :",
        error,
      );
    }
  };

  return (
    <div className="console-window">

      {/* ==================================================
          HEADER
          ================================================== */}

      <header className="console-header">

        <div className="console-title">

          <span
            className={`console-dot ${
              minecraftRunning
                ? "running"
                : "waiting"
            }`}
          />

          <span>
            ETERNIA CONSOLE
          </span>

          <span
            className={`console-status ${
              minecraftRunning
                ? "running"
                : "waiting"
            }`}
          >
            {minecraftRunning
              ? "MINECRAFT EN COURS"
              : "MINECRAFT ARRÊTÉ"}
          </span>

        </div>

        <div className="console-actions">

          {/* PID */}

          {minecraftPid !== null && (
            <span className="console-count">
              PID : {minecraftPid}
            </span>
          )}

          {/* TEMPS */}

          {minecraftRunning && (
            <span className="console-count">
              ⏱ {formatUptime(uptime)}
            </span>
          )}

          {/* NOMBRE DE LIGNES */}

          <span className="console-count">
            {filteredLines.length}
            {search
              ? ` / ${lines.length}`
              : ""}{" "}
            lignes
          </span>

          {/* AUTO SCROLL */}

          <button
            className={`console-button ${
              autoScroll ? "active" : ""
            }`}
            onClick={() =>
              setAutoScroll(
                (value) => !value,
              )
            }
            title="Activer ou désactiver le défilement automatique"
          >
            {autoScroll
              ? "AUTO-SCROLL"
              : "SCROLL MANUEL"}
          </button>

          {/* PAUSE */}

          <button
            className={`console-button ${
              paused
                ? "active pause"
                : ""
            }`}
            onClick={() =>
              setPaused(
                (value) => !value,
              )
            }
            title="Mettre l'affichage en pause"
          >
            {paused
              ? "REPRENDRE"
              : "PAUSE"}
          </button>

          {/* COPIER */}

          <button
            className="console-button"
            onClick={copyConsole}
            title="Copier tous les logs"
          >
            COPIER
          </button>

          {/* VIDER */}

          <button
            className="console-button clear"
            onClick={clearConsole}
            title="Vider la console"
          >
            VIDER
          </button>

        </div>
      </header>

      {/* ==================================================
          BARRE D'OUTILS
          ================================================== */}

      <div className="console-toolbar">

        <div className="console-search">

          <span className="console-search-icon">
            🔎
          </span>

          <input
            type="text"
            value={search}
            onChange={(event) =>
              setSearch(
                event.target.value,
              )
            }
            placeholder="Rechercher dans la console..."
            spellCheck={false}
          />

          {search && (
            <button
              className="console-search-clear"
              onClick={() =>
                setSearch("")
              }
              title="Effacer la recherche"
            >
              ×
            </button>
          )}

        </div>

        <div className="console-toolbar-status">

          <span className="status-item">
            <span className="status-dot white" />
            NORMAL
          </span>

          <span className="status-item">
            <span className="status-dot yellow" />
            INFO
          </span>

          <span className="status-item">
            <span className="status-dot red" />
            ERREUR
          </span>

        </div>

      </div>

      {/* ==================================================
          CONSOLE
          ================================================== */}

      <main className="console-body">

        {filteredLines.length === 0 ? (
          <div className="console-empty">
            {search
              ? "Aucun résultat."
              : "En attente des logs..."}
          </div>
        ) : (
          filteredLines.map(
            (line, index) => (
              <div
                key={index}
                className={`console-line ${line.kind}`}
              >

                <span className="console-prefix">
                  {line.kind === "error"
                    ? "[ERR]"
                    : line.kind === "info"
                      ? "[INFO]"
                      : "[MC]"}
                </span>

                <span className="console-message">
                  {line.message}
                </span>

              </div>
            ),
          )
        )}

        <div ref={endRef} />

      </main>

    </div>
  );
}

export default Console;