// ============================================================
// IMPORTS
// ============================================================

import { invoke } from "@tauri-apps/api/core";

import {
  Home,
  Newspaper,
  Server,
  Settings,
} from "lucide-react";

import {
  emit,
  listen,
} from "@tauri-apps/api/event";

import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

import {
  type PointerEvent,
  useEffect,
  useState,
} from "react";

import "./App.css";

// ============================================================
// IMAGES
// ============================================================

import fond1 from "./assets/images/fond1.png";
import fond2 from "./assets/images/fond2.png";
import fond3 from "./assets/images/fond3.png";
import fond4 from "./assets/images/fond4.png";
import iconpage from "./assets/images/iconpage.png";
import dragonGif from "./assets/images/dragon.gif";

// ============================================================
// CONSTANTES DU LAUNCHER
// ============================================================

const MINECRAFT_VERSION = "1.21.11";
const FABRIC_VERSION = "0.19.3";

const PROGRESS_TOTAL_STEPS = 9;

// ============================================================
// RAM
// ============================================================

const RAM_MIN_MB = 2048;
const RAM_STEP_MB = 512;

const DEFAULT_RAM_LIMIT_MB = 16384;

// ============================================================
// BACKGROUNDS
// ============================================================

const backgrounds = [
  fond1,
  fond2,
  fond3,
  fond4,
];

// ============================================================
// TYPES
// ============================================================

interface MinecraftSession {
  minecraft_uuid: string;
  minecraft_name: string;
  minecraft_access_token: string;
  refresh_token: string | null;
  expires_in: number | null;
}

interface MicrosoftDeviceCode {
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
  message?: string;
}

interface MinecraftStartedEvent {
  pid: number;
}

interface MinecraftStoppedEvent {
  pid: number;
  exit_code: number | null;
  success: boolean;
}

interface LauncherProgressEvent {
  step: number;
  total: number;
  message: string;
}

interface LauncherDownloadProgressEvent {
  current: number;
  total: number;
  percentage: number;
  message: string;
}

// ============================================================
// TYPE SLIDER RAM
// ============================================================

interface RamSliderProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
}

// ============================================================
// ETATS
// ============================================================

type MinecraftState =
  | "idle"
  | "preparing"
  | "running";

type LauncherPage =
  | "home"
  | "news"
  | "server"
  | "settings";

type SettingsCategory =
  | "general"
  | "minecraft";

// ============================================================
// ICONES COMPTE
// ============================================================

function UserIcon() {
  return (
    <svg
      width="19"
      height="19"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle
        cx="12"
        cy="9"
        r="4"
      />

      <path d="M4 21c0-4 3.5-7 9-7s9 3 9 7" />
    </svg>
  );
}

function LogoutIcon() {
  return (
    <svg
      width="19"
      height="19"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M10 17l5-5-5-5" />
      <path d="M15 12H3" />
      <path d="M21 3v19" />
    </svg>
  );
}

function SwitchAccountIcon() {
  return (
    <svg
      width="19"
      height="19"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M16 3h5v5" />
      <path d="M21 3l-7 7" />
      <path d="M9 21H3v-5" />
      <path d="M3 21l7-7" />
    </svg>
  );
}

function ChevronIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M6 9l6 6 6-6" />
    </svg>
  );
}

// ============================================================
// OUTILS RAM
// ============================================================

function formatRam(mb: number) {
  if (mb >= 1024) {
    const gb = mb / 1024;

    return Number.isInteger(gb)
      ? `${gb} Go`
      : `${gb.toFixed(1)} Go`;
  }

  return `${mb} Mo`;
}

function normalizeRamValue(mb: number) {
  const normalized =
    Math.floor(
      mb / RAM_STEP_MB,
    ) * RAM_STEP_MB;

  return Math.max(
    RAM_MIN_MB,
    normalized,
  );
}

// ============================================================
// SLIDER RAM
// ============================================================

function RamSlider({
  label,
  value,
  min,
  max,
  step,
  onChange,
}: RamSliderProps) {

  const safeMax =
    Math.max(
      min,
      max,
    );

  const safeValue =
    Math.min(
      Math.max(
        value,
        min,
      ),
      safeMax,
    );

  const percentage =
    safeMax > min
      ? (
          (safeValue - min) /
          (safeMax - min)
        ) * 100
      : 0;

  const updateFromPointer = (
    event: PointerEvent<HTMLDivElement>,
  ) => {

    const rect =
      event.currentTarget.getBoundingClientRect();

    if (rect.width <= 0) {
      return;
    }

    const position =
      Math.min(
        1,
        Math.max(
          0,
          (event.clientX - rect.left) /
            rect.width,
        ),
      );

    const rawValue =
      min +
      position *
        (safeMax - min);

    const steppedValue =
      Math.round(
        (rawValue - min) /
          step,
      ) * step + min;

    const nextValue =
      Math.min(
        safeMax,
        Math.max(
          min,
          steppedValue,
        ),
      );

    onChange(
      nextValue,
    );
  };

  return (
    <div className="ram-slider">

      <div className="ram-slider-header">

        <div className="ram-slider-info">

          <strong>
            {label}
          </strong>

          <span>
            Choisis la quantité maximale de RAM
            utilisée par Minecraft.
          </span>

        </div>

        <span className="ram-slider-value">
          {formatRam(safeValue)}
        </span>

      </div>

      <div
        className="ram-slider-track-area"

        onPointerDown={(event) => {

          event.currentTarget.setPointerCapture(
            event.pointerId,
          );

          updateFromPointer(
            event,
          );
        }}

        onPointerMove={(event) => {

          if (
            event.buttons === 1 &&
            event.currentTarget.hasPointerCapture(
              event.pointerId,
            )
          ) {

            updateFromPointer(
              event,
            );
          }
        }}

        onPointerUp={(event) => {

          if (
            event.currentTarget.hasPointerCapture(
              event.pointerId,
            )
          ) {

            event.currentTarget.releasePointerCapture(
              event.pointerId,
            );
          }
        }}

        role="slider"

        aria-label={label}

        aria-valuemin={min}

        aria-valuemax={safeMax}

        aria-valuenow={safeValue}

        aria-valuetext={formatRam(safeValue)}

        tabIndex={0}

        onKeyDown={(event) => {

          if (
            event.key === "ArrowLeft"
          ) {

            event.preventDefault();

            onChange(
              Math.max(
                min,
                safeValue - step,
              ),
            );
          }

          if (
            event.key === "ArrowRight"
          ) {

            event.preventDefault();

            onChange(
              Math.min(
                safeMax,
                safeValue + step,
              ),
            );
          }
        }}
      >

        <div className="ram-slider-track">

          <div className="ram-slider-rail" />

          <div
            className="ram-slider-fill"
            style={{
              width:
                `${percentage}%`,
            }}
          />

          <div
            className="ram-slider-thumb"
            style={{
              left:
                `${percentage}%`,
            }}
          />

        </div>

      </div>

      <div className="ram-slider-limits">

        <span>
          {formatRam(min)}
        </span>

        <span>
          {formatRam(safeMax)}
        </span>

      </div>

    </div>
  );
}

// ============================================================
// APP
// ============================================================

function App() {

  // ==========================================================
  // ETAT GENERAL
  // ==========================================================

  const [background, setBackground] =
    useState(0);

  const [currentPage, setCurrentPage] =
    useState<LauncherPage>("home");

  // ==========================================================
  // MICROSOFT
  // ==========================================================

  const [isLoggingIn, setIsLoggingIn] =
    useState(false);

  const [loginError, setLoginError] =
    useState<string | null>(null);

  const [session, setSession] =
    useState<MinecraftSession | null>(null);

  const [accountMenuOpen, setAccountMenuOpen] =
    useState(false);

  const [deviceCode, setDeviceCode] =
    useState<MicrosoftDeviceCode | null>(null);

  const [accounts, setAccounts] =
    useState<MinecraftSession[]>([]);

  // ==========================================================
  // MINECRAFT
  // ==========================================================

  const [minecraftState, setMinecraftState] =
    useState<MinecraftState>("idle");

  const [, setMinecraftPid] =
    useState<number | null>(null);

  // ==========================================================
  // PROGRESSION
  // ==========================================================

  const [progressStep, setProgressStep] =
    useState(0);

  const [progressMessage, setProgressMessage] =
    useState("");

  const [downloadCurrent, setDownloadCurrent] =
    useState(0);

  const [downloadTotal, setDownloadTotal] =
    useState(0);

  const [downloadPercentage, setDownloadPercentage] =
    useState(0);

  // ==========================================================
  // PARAMETRES
  // ==========================================================

  const [settingsCategory, setSettingsCategory] =
    useState<SettingsCategory>("general");

  const [startWithWindows, setStartWithWindows] =
    useState(false);

  const [autoUpdate, setAutoUpdate] =
    useState(true);

  const [showConsole, setShowConsole] =
    useState(true);

  // ==========================================================
  // REPARATION
  // ==========================================================

  const [isRepairing, setIsRepairing] =
    useState(false);

  // ==========================================================
  // RAM
  // ==========================================================

  const [totalRamMb, setTotalRamMb] =
    useState(
      DEFAULT_RAM_LIMIT_MB,
    );

  const [maxRam, setMaxRam] =
    useState(
      4096,
    );

  // ==========================================================
  // NAVIGATION
  // ==========================================================

  const navigateTo = (
    page: LauncherPage,
  ) => {

    setCurrentPage(page);

    setAccountMenuOpen(false);
  };

  // ==========================================================
  // RESET PROGRESSION
  // ==========================================================

  const resetProgress = () => {

    setProgressStep(0);

    setProgressMessage("");

    setDownloadCurrent(0);

    setDownloadTotal(0);

    setDownloadPercentage(0);
  };

  // ==========================================================
  // DETECTION AUTOMATIQUE DE LA RAM
  // ==========================================================

  useEffect(() => {

    let mounted = true;

    const detectSystemRam =
      async () => {

        try {

          const detectedRamMb =
            await invoke<number>(
              "get_system_ram",
            );

          if (!mounted) {
            return;
          }

          if (
            !Number.isFinite(
              detectedRamMb,
            ) ||
            detectedRamMb <= 0
          ) {

            return;
          }

          const ramLimit =
            Math.max(
              RAM_MIN_MB,
              Math.floor(
                detectedRamMb,
              ),
            );

          setTotalRamMb(
            ramLimit,
          );

          setMaxRam(
            (currentMax) => {

              const normalizedCurrent =
                normalizeRamValue(
                  currentMax,
                );

              return Math.min(
                ramLimit,
                Math.max(
                  RAM_MIN_MB,
                  normalizedCurrent,
                ),
              );
            },
          );

          console.log(
            "========================================",
          );

          console.log(
            "RAM SYSTÈME DÉTECTÉE",
          );

          console.log(
            `RAM totale : ${detectedRamMb} MB`,
          );

          console.log(
            `RAM disponible : ${formatRam(ramLimit)}`,
          );

          console.log(
            `RAM minimum Minecraft : ${formatRam(RAM_MIN_MB)}`,
          );

          console.log(
            "========================================",
          );

        } catch (error) {

          console.error(
            "Impossible de détecter la RAM du PC :",
            error,
          );
        }
      };

    void detectSystemRam();

    return () => {

      mounted = false;

    };

  }, []);

  // ==========================================================
  // LISTENERS LAUNCHER
  // ==========================================================

  useEffect(() => {

    let mounted = true;

    const unlistenProgress =
      listen<LauncherProgressEvent>(
        "launcher-progress",
        (event) => {

          if (!mounted) {
            return;
          }

          setMinecraftState(
            (current) =>
              current === "running"
                ? current
                : "preparing",
          );

          setProgressStep(
            event.payload.step,
          );

          setProgressMessage(
            event.payload.message,
          );
        },
      );

    const unlistenDownload =
      listen<LauncherDownloadProgressEvent>(
        "launcher-download-progress",
        (event) => {

          if (!mounted) {
            return;
          }

          setMinecraftState(
            (current) =>
              current === "running"
                ? current
                : "preparing",
          );

          setDownloadCurrent(
            event.payload.current,
          );

          setDownloadTotal(
            event.payload.total,
          );

          setDownloadPercentage(
            event.payload.percentage,
          );

          setProgressMessage(
            event.payload.message,
          );
        },
      );

    const unlistenStarted =
      listen<MinecraftStartedEvent>(
        "minecraft-started",
        (event) => {

          if (!mounted) {
            return;
          }

          console.log(
            "========================================",
          );

          console.log(
            "MINECRAFT LANCÉ",
          );

          console.log(
            "PID :",
            event.payload.pid,
          );

          console.log(
            "========================================",
          );

          setMinecraftPid(
            event.payload.pid,
          );

          setMinecraftState(
            "running",
          );

          setProgressStep(
            PROGRESS_TOTAL_STEPS,
          );

          setProgressMessage("");

          setDownloadCurrent(0);

          setDownloadTotal(0);

          setDownloadPercentage(0);
        },
      );

    const unlistenStopped =
      listen<MinecraftStoppedEvent>(
        "minecraft-stopped",
        (event) => {

          if (!mounted) {
            return;
          }

          console.log(
            "========================================",
          );

          console.log(
            "MINECRAFT ARRÊTÉ",
          );

          console.log(
            "PID :",
            event.payload.pid,
          );

          console.log(
            "Code :",
            event.payload.exit_code,
          );

          console.log(
            "Succès :",
            event.payload.success,
          );

          console.log(
            "========================================",
          );

          setMinecraftPid(null);

          setMinecraftState(
            "idle",
          );

          resetProgress();
        },
      );

    return () => {

      mounted = false;

      void unlistenProgress.then(
        (fn) => fn(),
      );

      void unlistenDownload.then(
        (fn) => fn(),
      );

      void unlistenStarted.then(
        (fn) => fn(),
      );

      void unlistenStopped.then(
        (fn) => fn(),
      );
    };

  }, []);

  // ==========================================================
  // JAVA
  // ==========================================================

  useEffect(() => {

    const checkJava =
      async () => {

        try {

          const version =
            await invoke<string>(
              "get_java_version",
            );

          console.log(
            "========================================",
          );

          console.log(
            "JAVA DÉTECTÉ",
          );

          console.log(version);

          console.log(
            "========================================",
          );

        } catch (error) {

          console.error(
            "Impossible de détecter Java :",
            error,
          );
        }
      };

    void checkJava();

  }, []);

  // ==========================================================
  // SESSION + COMPTES
  // ==========================================================

  useEffect(() => {

    const loadSessionAndAccounts =
      async () => {

        try {

          const [
            currentSession,
            savedAccounts,
          ] = await Promise.all([

            invoke<
              MinecraftSession | null
            >(
              "get_session",
            ),

            invoke<MinecraftSession[]>(
              "get_accounts",
            ),
          ]);

          console.log(
            "Session Minecraft :",
            currentSession,
          );

          console.log(
            "Comptes sauvegardés :",
            savedAccounts,
          );

          setSession(
            currentSession,
          );

          setAccounts(
            savedAccounts,
          );

        } catch (error) {

          console.error(
            "Impossible de charger la session et les comptes :",
            error,
          );
        }
      };

    void loadSessionAndAccounts();

  }, []);

  // ==========================================================
  // MICROSOFT DEVICE CODE
  // ==========================================================

  useEffect(() => {

    const unlisten =
      listen<MicrosoftDeviceCode>(
        "microsoft-device-code",
        (event) => {

          setDeviceCode(
            event.payload,
          );
        },
      );

    return () => {

      void unlisten.then(
        (fn) => fn(),
      );
    };

  }, []);

  // ==========================================================
  // BACKGROUND
  // ==========================================================

  useEffect(() => {

    const timer =
      setInterval(() => {

        setBackground(
          (current) =>
            (current + 1) %
            backgrounds.length,
        );

      }, 9000);

    return () => {

      clearInterval(timer);

    };

  }, []);

  // ==========================================================
  // CONSOLE ETERNIA
  // ==========================================================

  const openConsoleWindow =
    async () => {

      try {

        console.log(
          "========================================",
        );

        console.log(
          "OUVERTURE CONSOLE ETERNIA",
        );

        console.log(
          "========================================",
        );

        const consoleWindow =
          await WebviewWindow.getByLabel(
            "console",
          );

        if (!consoleWindow) {

          console.error(
            "ERREUR : la fenêtre 'console' n'existe pas.",
          );

          return;
        }

        const visible =
          await consoleWindow.isVisible();

        if (!visible) {

          await consoleWindow.show();

        }

        await consoleWindow.setFocus();

        await emit(
          "launcher-console-clear",
        );

        console.log(
          "Console Eternia affichée et active.",
        );

      } catch (error) {

        console.error(
          "ERREUR CONSOLE ETERNIA :",
          error,
        );
      }
    };

  // ==========================================================
  // CONNEXION MICROSOFT
  // ==========================================================

  const handleMicrosoftLogin =
    async () => {

      if (isLoggingIn) {
        return;
      }

      setIsLoggingIn(true);

      setLoginError(null);

      try {

        const result =
          await invoke<MinecraftSession>(
            "microsoft_login",
            {
              forceNewAccount: false,
            },
          );

        console.log(
          "Connexion Microsoft :",
          result,
        );

        const [
          currentSession,
          savedAccounts,
        ] = await Promise.all([

          invoke<
            MinecraftSession | null
          >(
            "get_session",
          ),

          invoke<MinecraftSession[]>(
            "get_accounts",
          ),
        ]);

        setSession(
          currentSession,
        );

        setAccounts(
          savedAccounts,
        );

        setDeviceCode(null);

        setAccountMenuOpen(false);

      } catch (error) {

        console.error(
          "Erreur Microsoft :",
          error,
        );

        setLoginError(
          typeof error === "string"
            ? error
            : "Une erreur est survenue pendant la connexion Microsoft.",
        );

      } finally {

        setIsLoggingIn(false);

      }
    };

  // ==========================================================
  // ANNULER PREPARATION
  // ==========================================================

  const handleCancelLaunch =
    async () => {

      if (
        minecraftState !==
        "preparing"
      ) {

        return;

      }

      try {

        await invoke(
          "cancel_launch",
        );

        setMinecraftState(
          "idle",
        );

        resetProgress();

      } catch (error) {

        console.error(
          "Erreur lors de l'annulation :",
          error,
        );

        setLoginError(
          typeof error === "string"
            ? error
            : "Impossible d'annuler la préparation.",
        );
      }
    };

  // ==========================================================
  // REPARER L'INSTALLATION MINECRAFT
  // ==========================================================

  const handleRepairInstallation =
    async () => {

      if (isRepairing) {
        return;
      }

      if (
        minecraftState === "preparing" ||
        minecraftState === "running"
      ) {

        setLoginError(
          "Impossible de réparer Minecraft pendant que le jeu est lancé ou en préparation.",
        );

        return;
      }

      setIsRepairing(true);

      setLoginError(null);

      resetProgress();

      setMinecraftState(
        "preparing",
      );

      setProgressMessage(
        "Préparation de la réparation...",
      );

      try {

        if (showConsole) {

          await openConsoleWindow();

        }

        console.log(
          "========================================",
        );

        console.log(
          "ETERNIA - RÉPARATION DE L'INSTALLATION",
        );

        console.log(
          "========================================",
        );

        const result =
          await invoke<string>(
            "repair_installation",
          );

        console.log(
          "========================================",
        );

        console.log(
          "RÉPARATION TERMINÉE",
        );

        console.log(result);

        setProgressStep(
          PROGRESS_TOTAL_STEPS,
        );

        setProgressMessage(
          "Installation réparée.",
        );

      } catch (error) {

        console.error(
          "Erreur pendant la réparation Minecraft :",
          error,
        );

        setLoginError(
          error instanceof Error
            ? error.message
            : String(error),
        );

      } finally {

        setIsRepairing(false);

        setMinecraftState(
          "idle",
        );

        setTimeout(() => {

          resetProgress();

        }, 1200);

      }
    };


  // ==========================================================
  // JOUER
  // ==========================================================

  const handlePlay =
    async () => {

      // --------------------------------------------------------
      // ANNULER UNE PREPARATION
      // --------------------------------------------------------

      if (
        minecraftState ===
        "preparing"
      ) {

        await handleCancelLaunch();

        return;
      }

      // --------------------------------------------------------
      // ARRETER MINECRAFT
      // --------------------------------------------------------

      if (
        minecraftState ===
        "running"
      ) {

        try {

          await invoke(
            "stop_minecraft",
          );

        } catch (error) {

          setLoginError(
            error instanceof Error
              ? error.message
              : String(error),
          );
        }

        return;
      }

      // --------------------------------------------------------
      // CONNEXION EN COURS
      // --------------------------------------------------------

      if (isLoggingIn) {
        return;
      }

      // --------------------------------------------------------
      // PAS DE SESSION
      // --------------------------------------------------------

      if (!session) {

        await handleMicrosoftLogin();

        return;
      }

      // --------------------------------------------------------
      // PREPARATION
      // --------------------------------------------------------

      setMinecraftState(
        "preparing",
      );

      setLoginError(null);

      resetProgress();

      setProgressMessage(
        "Vérification de la connexion Microsoft...",
      );

      if (showConsole) {

        await openConsoleWindow();

      }

      try {

        console.log(
          "========================================",
        );

        console.log(
          "ETERNIA - VÉRIFICATION DE LA CONNEXION",
        );

        console.log(
          "========================================",
        );

        await invoke(
          "microsoft_login",
          {
            forceNewAccount: false,
          },
        );

        const refreshedSession =
          await invoke<
            MinecraftSession | null
          >(
            "get_session",
          );

        setSession(
          refreshedSession,
        );

        setProgressMessage(
          "Connexion Microsoft vérifiée...",
        );

        console.log(
          "Connexion Microsoft vérifiée.",
        );

        console.log(
          "========================================",
        );

        console.log(
          "ETERNIA - PRÉPARATION AUTOMATIQUE",
        );

        console.log(
          "========================================",
        );

        const safeMaxRam =
          Math.min(
            Math.max(
              RAM_MIN_MB,
              normalizeRamValue(
                maxRam,
              ),
            ),
            Math.max(
              RAM_MIN_MB,
              totalRamMb,
            ),
          );

        setMaxRam(
          safeMaxRam,
        );

        console.log(
          "RAM Minecraft sélectionnée :",
          formatRam(safeMaxRam),
        );

        console.log(
          "RAM Minecraft en MB :",
          safeMaxRam,
        );

        const result =
          await invoke<string>(
            "prepare_and_launch",
            {
              maxRamMb:
                safeMaxRam,
            },
          );

        console.log(
          "========================================",
        );

        console.log(
          "MINECRAFT",
        );

        console.log(
          "========================================",
        );

        console.log(result);

      } catch (error) {

        console.error(
          "Erreur préparation Minecraft :",
          error,
        );

        setLoginError(
          error instanceof Error
            ? error.message
            : String(error),
        );

        setMinecraftState(
          "idle",
        );

        setMinecraftPid(null);

        resetProgress();

      }
    };

  // ==========================================================
  // DECONNEXION
  // ==========================================================

  const handleLogout =
    async () => {

      if (!session) {
        return;
      }

      try {

        await invoke(
          "remove_account",
          {
            uuid:
              session.minecraft_uuid,
          },
        );

        setSession(null);

        setAccountMenuOpen(false);

        setLoginError(null);

        const savedAccounts =
          await invoke<MinecraftSession[]>(
            "get_accounts",
          );

        setAccounts(
          savedAccounts,
        );

        console.log(
          "Compte supprimé.",
        );

      } catch (error) {

        console.error(
          "Erreur pendant la suppression du compte :",
          error,
        );

        setLoginError(
          typeof error === "string"
            ? error
            : "Impossible de supprimer le compte.",
        );
      }
    };

  // ==========================================================
  // CHANGER DE COMPTE
  // ==========================================================

  const handleChangeAccount =
    async () => {

      setAccountMenuOpen(false);

      setLoginError(null);

      setIsLoggingIn(true);

      try {

        const result =
          await invoke<MinecraftSession>(
            "microsoft_login",
            {
              forceNewAccount: true,
            },
          );

        console.log(
          "Nouveau compte Microsoft :",
          result,
        );

        const [
          currentSession,
          savedAccounts,
        ] = await Promise.all([

          invoke<
            MinecraftSession | null
          >(
            "get_session",
          ),

          invoke<MinecraftSession[]>(
            "get_accounts",
          ),
        ]);

        setSession(
          currentSession,
        );

        setAccounts(
          savedAccounts,
        );

        setDeviceCode(null);

      } catch (error) {

        console.error(
          "Erreur changement de compte :",
          error,
        );

        setLoginError(
          typeof error === "string"
            ? error
            : "Une erreur est survenue pendant le changement de compte.",
        );

      } finally {

        setIsLoggingIn(false);

      }
    };

  // ==========================================================
  // CHANGER DE COMPTE SAUVEGARDE
  // ==========================================================

  const handleSelectAccount =
    async (
      account: MinecraftSession,
    ) => {

      try {

        await invoke(
          "select_account",
          {
            uuid:
              account.minecraft_uuid,
          },
        );

        const currentSession =
          await invoke<
            MinecraftSession | null
          >(
            "get_session",
          );

        setSession(
          currentSession,
        );

        setAccountMenuOpen(false);

        setLoginError(null);

      } catch (error) {

        console.error(
          "Erreur lors du changement de compte :",
          error,
        );

        setLoginError(
          typeof error === "string"
            ? error
            : "Impossible de changer de compte.",
        );
      }
    };

  // ==========================================================
  // PROGRESSION GLOBALE
  // ==========================================================

  const progressPercentage =
    downloadTotal > 0

      ? Math.min(
          100,
          Math.max(
            0,
            downloadPercentage,
          ),
        )

      : Math.min(
          100,
          Math.max(
            0,
            Math.round(
              (
                progressStep /
                PROGRESS_TOTAL_STEPS
              ) * 100,
            ),
          ),
        );

  // ==========================================================
  // JSX
  // ==========================================================

  return (
    <div className="launcher">

      {/* ======================================================
          MODAL MICROSOFT
          ====================================================== */}

      {deviceCode && (

        <div className="microsoft-modal-backdrop">

          <div className="microsoft-modal">

            <h2>
              Connexion Microsoft
            </h2>

            <p>
              Ouvre Microsoft et entre
              le code suivant :
            </p>

            <div className="microsoft-code">
              {deviceCode.user_code}
            </div>

            <p className="microsoft-modal-info">
              Une fenêtre Microsoft a été
              ouverte.
              <br />
              Termine la connexion puis
              reviens ici.
            </p>

            <div className="microsoft-actions">

              <button
                onClick={() => {

                  void navigator.clipboard.writeText(
                    deviceCode.user_code,
                  );

                }}
              >
                COPIER
              </button>

              <button
                onClick={() => {

                  window.open(
                    deviceCode.verification_uri,
                    "_blank",
                  );

                }}
              >
                OUVRIR MICROSOFT
              </button>

              <button
                className="cancel"
                onClick={async () => {

                  try {

                    await invoke(
                      "cancel_microsoft_login",
                    );

                  } catch (error) {

                    console.error(
                      "Erreur lors de l'annulation Microsoft :",
                      error,
                    );

                  } finally {

                    setDeviceCode(null);

                    setIsLoggingIn(false);

                  }

                }}
              >
                ANNULER
              </button>

            </div>

          </div>

        </div>

      )}

      {/* ======================================================
          BACKGROUNDS
          ====================================================== */}

      {backgrounds.map(
        (image, index) => (

          <div
            key={image}
            className={
              index === background
                ? "background background-active"
                : "background"
            }
            style={{
              backgroundImage:
                `url(${image})`,
            }}
          />

        ),
      )}

      <div className="overlay" />

      {/* ======================================================
          HEADER
          ====================================================== */}

      <header className="header">

        <div className="brand">

          <img
            src={iconpage}
            alt="Eternia"
            className="brand-icon"
          />

          <div>

            <div className="brand-name">
              ETERNIA
            </div>

            <div className="brand-subtitle">
              PIKMIN ADVENTURE
            </div>

          </div>

        </div>

        {/* ====================================================
            NAVIGATION
            ==================================================== */}

        <nav className="navigation">

          <button
            className={
              currentPage === "home"
                ? "nav-button active"
                : "nav-button"
            }
            onClick={() =>
              navigateTo("home")
            }
          >

            <Home
              size={18}
              strokeWidth={2}
            />

            <span>
              ACCUEIL
            </span>

          </button>

          <button
            className={
              currentPage === "news"
                ? "nav-button active"
                : "nav-button"
            }
            onClick={() =>
              navigateTo("news")
            }
          >

            <Newspaper
              size={18}
              strokeWidth={2}
            />

            <span>
              ACTUALITÉS
            </span>

          </button>

          <button
            className={
              currentPage === "server"
                ? "nav-button active"
                : "nav-button"
            }
            onClick={() =>
              navigateTo("server")
            }
          >

            <Server
              size={18}
              strokeWidth={2}
            />

            <span>
              SERVEUR
            </span>

          </button>

          <button
            className={
              currentPage === "settings"
                ? "nav-button active"
                : "nav-button"
            }
            onClick={() =>
              navigateTo("settings")
            }
          >

            <Settings
              size={18}
              strokeWidth={2}
            />

            <span>
              PARAMÈTRES
            </span>

          </button>

        </nav>

        {/* ====================================================
            COMPTE
            ==================================================== */}

        <button
          className={
            accountMenuOpen
              ? "nav-button account-nav active"
              : "nav-button account-nav"
          }
          onClick={() =>
            setAccountMenuOpen(
              (current) => !current,
            )
          }
        >

          <UserIcon />

          <span>
            {session
              ? session.minecraft_name
              : "COMPTE"}
          </span>

          <ChevronIcon />

        </button>

        {/* ====================================================
            MENU COMPTE
            ==================================================== */}

        {accountMenuOpen && (

          <div className="account-menu">

            {accounts.length > 0 && (

              <div className="saved-accounts">

                {accounts.map(
                  (account) => (

                    <button
                      key={
                        account.minecraft_uuid
                      }
                      className={
                        session?.minecraft_uuid ===
                        account.minecraft_uuid
                          ? "saved-account active"
                          : "saved-account"
                      }
                      onClick={() =>
                        void handleSelectAccount(
                          account,
                        )
                      }
                    >

                      <div className="saved-account-avatar">

                        <img
                          src={`https://mc-heads.net/avatar/${encodeURIComponent(
                            account.minecraft_name,
                          )}/40`}
                          alt={`Avatar de ${account.minecraft_name}`}
                        />

                      </div>

                      <div className="saved-account-info">

                        <strong>
                          {
                            account.minecraft_name
                          }
                        </strong>

                        <span>
                          Compte Minecraft
                        </span>

                      </div>

                      {session?.minecraft_uuid ===
                        account.minecraft_uuid && (

                        <span className="saved-account-check">
                          ✓
                        </span>

                      )}

                    </button>

                  ),
                )}

              </div>

            )}

            {session ? (

              <>

                <div className="account-divider" />

                <button
                  className="account-action"
                  onClick={() =>
                    void handleChangeAccount()
                  }
                >

                  <SwitchAccountIcon />

                  <span>
                    CHANGER DE COMPTE
                  </span>

                </button>

                <button
                  className="account-action account-action-danger"
                  onClick={() =>
                    void handleLogout()
                  }
                >

                  <LogoutIcon />

                  <span>
                    DÉCONNEXION
                  </span>

                </button>

              </>

            ) : (

              <button
                className="account-action"
                onClick={() =>
                  void handleMicrosoftLogin()
                }
                disabled={isLoggingIn}
              >

                <UserIcon />

                <span>
                  {isLoggingIn
                    ? "CONNEXION..."
                    : "SE CONNECTER"}
                </span>

              </button>

            )}

          </div>

        )}

      </header>

      {/* ======================================================
          ACCUEIL
          ====================================================== */}

      {currentPage === "home" && (

        <main className="content">

          <section className="hero">

            <div className="welcome">
              BIENVENUE SUR
            </div>

            <h1>
              ETERNIA
            </h1>

            <p className="description">
              Pars à l'aventure dans un
              monde rempli de découvertes,
              <br />
              de créatures et de mystères.
            </p>

            <button
              className={
                minecraftState ===
                "preparing"
                  ? "play-button cancel"
                  : minecraftState ===
                      "running"
                    ? "play-button running"
                    : "play-button"
              }
              onClick={() =>
                void handlePlay()
              }
              disabled={isLoggingIn}
            >

              <span className="play-icon">

                {isLoggingIn
                  ? "◌"
                  : minecraftState ===
                      "preparing"
                    ? "×"
                    : minecraftState ===
                        "running"
                      ? "■"
                      : "▶"}

              </span>

              {isLoggingIn
                ? "CONNEXION..."
                : minecraftState ===
                    "preparing"
                  ? "CANCEL"
                  : minecraftState ===
                      "running"
                    ? "ARRÊTER"
                    : session
                      ? `JOUER — ${session.minecraft_name}`
                      : "JOUER"}

            </button>

            {/* ==================================================
                PROGRESSION
                ================================================== */}

            {minecraftState ===
              "preparing" && (

              <div className="launcher-progress">

                <div className="launcher-progress-text">

                  <span>
                    {progressMessage}
                  </span>

                  <span>
                    {progressPercentage} %
                  </span>

                </div>

                <div className="launcher-progress-bar">

                  <div
                    className="launcher-progress-fill"
                    style={{
                      width:
                        `${progressPercentage}%`,
                    }}
                  />

                  <img
                    src={dragonGif}
                    alt="Dragon"
                    className="progress-dragon"
                    style={{
                      left:
                        `${Math.min(
                          96,
                          Math.max(
                            4,
                            progressPercentage,
                          ),
                        )}%`,
                    }}
                  />

                </div>

                <div className="launcher-progress-step">

                  {downloadTotal > 0
                    ? `${downloadCurrent} / ${downloadTotal} fichiers`
                    : `Étape ${progressStep}/${PROGRESS_TOTAL_STEPS}`}

                </div>

              </div>

            )}

            {/* ==================================================
                ERREUR
                ================================================== */}

            {loginError !== null && (

              <div className="login-error">
                {loginError}
              </div>

            )}

            {/* ==================================================
                COMPTE CONNECTE
                ================================================== */}

            {session !== null && (

              <div className="minecraft-account">

                <div className="minecraft-avatar-small">

                  <img
                    src={`https://mc-heads.net/avatar/${encodeURIComponent(
                      session.minecraft_name,
                    )}/40`}
                    alt={`Avatar de ${session.minecraft_name}`}
                  />

                </div>

                <div>

                  <strong>
                    {
                      session.minecraft_name
                    }
                  </strong>

                  <span>
                    Compte Minecraft connecté
                  </span>

                </div>

              </div>

            )}

            {/* ==================================================
                SERVEUR
                ================================================== */}

            <div className="server-card">

              <span className="status-dot" />

              <div>

                <strong>
                  Serveur Eternia
                </strong>

                <span>
                  En ligne
                </span>

              </div>

              <div className="players">

                <strong>
                  0
                </strong>

                <span>
                  joueurs
                </span>

              </div>

            </div>

          </section>

          {/* ==================================================
              ACTUALITES
              ================================================== */}

          <section className="news">

            <div className="section-title">

              <span>
                DERNIÈRES ACTUALITÉS
              </span>

              <button
                onClick={() =>
                  navigateTo("news")
                }
              >
                VOIR TOUT →
              </button>

            </div>

            <div className="news-card">

              <div>

                <span className="news-tag">
                  ETERNIA
                </span>

                <h2>
                  Bienvenue sur Eternia
                </h2>

                <p>
                  Prépare-toi à découvrir
                  une nouvelle aventure
                  Minecraft.
                </p>

              </div>

              <span className="news-date">
                25 SEPT. 2026
              </span>

            </div>

          </section>

        </main>

      )}

      {/* ======================================================
          ACTUALITES
          ====================================================== */}

      {currentPage === "news" && (

        <main className="news-page">

          <div className="settings-header">

            <div>

              <span className="settings-eyebrow">
                ETERNIA
              </span>

              <h1>
                Actualités
              </h1>

              <p>
                Découvre les dernières
                nouvelles d'Eternia.
              </p>

            </div>

          </div>

          <div className="news-card">

            <div>

              <span className="news-tag">
                ETERNIA
              </span>

              <h2>
                Bienvenue sur Eternia
              </h2>

              <p>
                Prépare-toi à découvrir
                une nouvelle aventure
                Minecraft.
              </p>

            </div>

            <span className="news-date">
              25 SEPT. 2026
            </span>

          </div>

        </main>

      )}

      {/* ======================================================
          SERVEUR
          ====================================================== */}

      {currentPage === "server" && (

        <main className="server-page">

          <div className="settings-header">

            <div>

              <span className="settings-eyebrow">
                ETERNIA
              </span>

              <h1>
                Serveur
              </h1>

              <p>
                Informations sur le serveur
                Minecraft Eternia.
              </p>

            </div>

          </div>

          <div className="server-card">

            <span className="status-dot" />

            <div>

              <strong>
                Serveur Eternia
              </strong>

              <span>
                En ligne
              </span>

            </div>

            <div className="players">

              <strong>
                0
              </strong>

              <span>
                joueurs
              </span>

            </div>

          </div>

        </main>

      )}

      {/* ======================================================
          PARAMETRES
          ====================================================== */}

      {currentPage === "settings" && (

        <main className="settings-page">

          <div className="settings-layout">

            {/* ==================================================
                MENU CATEGORIES
                ================================================== */}

            <aside className="settings-sidebar">

              <div className="settings-sidebar-title">
                PARAMÈTRES
              </div>

              {/* GENERAL */}

              <button
                className={
                  settingsCategory ===
                  "general"
                    ? "settings-category active"
                    : "settings-category"
                }
                onClick={() =>
                  setSettingsCategory(
                    "general",
                  )
                }
              >

                <span className="settings-category-icon">
                  ⚙
                </span>

                <span>
                  Général
                </span>

              </button>

              {/* MINECRAFT */}

              <button
                className={
                  settingsCategory ===
                  "minecraft"
                    ? "settings-category active"
                    : "settings-category"
                }
                onClick={() =>
                  setSettingsCategory(
                    "minecraft",
                  )
                }
              >

                <span className="settings-category-icon">
                  ⛏
                </span>

                <span>
                  Minecraft
                </span>

              </button>

              {/* AFFICHAGE */}

              <button
                className="settings-category disabled"
                disabled
              >

                <span className="settings-category-icon">
                  🖥
                </span>

                <span>
                  Affichage
                </span>

                <small>
                  Bientôt
                </small>

              </button>

              {/* LAUNCHER */}

              <button
                className="settings-category disabled"
                disabled
              >

                <span className="settings-category-icon">
                  🚀
                </span>

                <span>
                  Launcher
                </span>

                <small>
                  Bientôt
                </small>

              </button>

              {/* JEU */}

              <button
                className="settings-category disabled"
                disabled
              >

                <span className="settings-category-icon">
                  🎮
                </span>

                <span>
                  Jeu
                </span>

                <small>
                  Bientôt
                </small>

              </button>

              {/* AVANCE */}

              <button
                className="settings-category disabled"
                disabled
              >

                <span className="settings-category-icon">
                  🔧
                </span>

                <span>
                  Avancé
                </span>

                <small>
                  Bientôt
                </small>

              </button>

              <div className="settings-sidebar-separator" />

              {/* DISCORD */}

              <button
                className="settings-discord"
                onClick={() =>
                  window.open(
                    "https://discord.gg/vZMCBmrDwu",
                    "_blank",
                  )
                }
              >

                <span className="settings-category-icon">
                  💬
                </span>

                <span>
                  DISCORD
                </span>

              </button>

            </aside>

            {/* ==================================================
                CONTENU
                ================================================== */}

            <section className="settings-content">

              {/* TITRE */}

              <div className="settings-header">

                <div>

                  <span className="settings-eyebrow">
                    ETERNIA
                  </span>

                  <h1>
                    Paramètres
                  </h1>

                  <p>
                    Configure ton expérience
                    Eternia.
                  </p>

                </div>

              </div>

              {/* =================================================
                  GENERAL
                  ================================================= */}

              {settingsCategory ===
                "general" && (

                <div className="settings-panel">

                  <div className="settings-panel-header">

                    <div>

                      <span className="settings-panel-kicker">
                        CONFIGURATION
                      </span>

                      <h2>
                        Général
                      </h2>

                      <p>
                        Les paramètres généraux
                        du launcher.
                      </p>

                    </div>

                  </div>

                  <div className="settings-options">

                    {/* WINDOWS */}

                    <div className="settings-option">

                      <div className="settings-option-info">

                        <strong>
                          Démarrer avec Windows
                        </strong>

                        <span>
                          Lance automatiquement
                          Eternia lorsque Windows
                          démarre.
                        </span>

                      </div>

                      <button
                        className={
                          startWithWindows
                            ? "settings-toggle active"
                            : "settings-toggle"
                        }
                        onClick={() =>
                          setStartWithWindows(
                            (value) =>
                              !value,
                          )
                        }
                        aria-label="Démarrer avec Windows"
                        aria-pressed={
                          startWithWindows
                        }
                      >

                        <span />

                      </button>

                    </div>

                    {/* MISES A JOUR */}

                    <div className="settings-option">

                      <div className="settings-option-info">

                        <strong>
                          Mises à jour automatiques
                        </strong>

                        <span>
                          Vérifie automatiquement
                          si une nouvelle version
                          d'Eternia est disponible.
                        </span>

                      </div>

                      <button
                        className={
                          autoUpdate
                            ? "settings-toggle active"
                            : "settings-toggle"
                        }
                        onClick={() =>
                          setAutoUpdate(
                            (value) =>
                              !value,
                          )
                        }
                        aria-label="Mises à jour automatiques"
                        aria-pressed={
                          autoUpdate
                        }
                      >

                        <span />

                      </button>

                    </div>

                    {/* CONSOLE */}

                    <div className="settings-option">

                      <div className="settings-option-info">

                        <strong>
                          Console du launcher
                        </strong>

                        <span>
                          Affiche la console
                          détaillée pendant le
                          téléchargement et le
                          lancement.
                        </span>

                      </div>

                      <button
                        className={
                          showConsole
                            ? "settings-toggle active"
                            : "settings-toggle"
                        }
                        onClick={() =>
                          setShowConsole(
                            (value) =>
                              !value,
                          )
                        }
                        aria-label="Console du launcher"
                        aria-pressed={
                          showConsole
                        }
                      >

                        <span />

                      </button>

                    </div>

                  </div>

                </div>

              )}

              {/* =================================================
                  MINECRAFT
                  ================================================= */}

              {settingsCategory ===
                "minecraft" && (

                <div className="settings-panel">

                  <div className="settings-panel-header">

                    <div>

                      <span className="settings-panel-kicker">
                        CONFIGURATION
                      </span>

                      <h2>
                        Minecraft
                      </h2>

                      <p>
                        Configure l'installation
                        et les ressources utilisées
                        par Minecraft.
                      </p>

                    </div>

                  </div>

                  <div className="settings-options">

                    {/* VERSION */}

                    <div className="settings-option">

                      <div className="settings-option-info">

                        <strong>
                          Version Minecraft
                        </strong>

                        <span>
                          Version utilisée par
                          Eternia.
                        </span>

                      </div>

                      <div className="settings-value">
                        {MINECRAFT_VERSION}
                      </div>

                    </div>

                    {/* FABRIC */}

                    <div className="settings-option">

                      <div className="settings-option-info">

                        <strong>
                          Fabric Loader
                        </strong>

                        <span>
                          Chargeur de mods utilisé
                          par Eternia.
                        </span>

                      </div>

                      <div className="settings-value">
                        {FABRIC_VERSION}
                      </div>

                    </div>

                    {/* RAM DISPONIBLE */}

                    <div className="settings-option">

                      <div className="settings-option-info">

                        <strong>
                          RAM disponible
                        </strong>

                        <span>
                          Mémoire physique détectée
                          sur ce PC.
                        </span>

                      </div>

                      <div className="settings-value">
                        {formatRam(totalRamMb)}
                      </div>

                    </div>

                    {/* RAM MAXIMUM */}

                    <div className="settings-option settings-option-column">

                      <RamSlider
                        label="RAM maximum"
                        value={maxRam}
                        min={RAM_MIN_MB}
                        max={Math.max(
                          RAM_MIN_MB,
                          totalRamMb,
                        )}
                        step={RAM_STEP_MB}
                        onChange={(value) => {

                          const normalizedValue =
                            normalizeRamValue(
                              value,
                            );

                          const safeValue =
                            Math.min(
                              Math.max(
                                RAM_MIN_MB,
                                normalizedValue,
                              ),
                              Math.max(
                                RAM_MIN_MB,
                                totalRamMb,
                              ),
                            );

                          setMaxRam(
                            safeValue,
                          );
                        }}
                      />

                    </div>

                    {/* DOSSIER MINECRAFT */}

                    <div className="settings-option">

                      <div className="settings-option-info">

                        <strong>
                          Dossier Minecraft
                        </strong>

                        <span>
                          Ouvre le dossier utilisé
                          par Eternia pour
                          Minecraft.
                        </span>

                      </div>

                      <button
                        className="settings-action-button"
                        onClick={async () => {

                          try {

                            const path =
                              await invoke<string>(
                                "get_minecraft_dir",
                              );

                            console.log(
                              "Dossier Minecraft :",
                              path,
                            );

                            await invoke(
                              "open_minecraft_dir",
                            );

                          } catch (error) {

                            console.error(
                              "Impossible d'ouvrir le dossier Minecraft :",
                              error,
                            );

                          }

                        }}
                      >
                        OUVRIR
                      </button>

                    </div>

                    {/* REPARER L'INSTALLATION */}

                    <div className="settings-option">

                      <div className="settings-option-info">

                        <strong>
                          Réparer l'installation
                        </strong>

                        <span>
                          Vérifie et réinstalle les
                          fichiers Minecraft nécessaires
                          à Eternia.
                        </span>

                      </div>

                      <button
                        className="settings-action-button"
                        onClick={() =>
                          void handleRepairInstallation()
                        }
                        disabled={
                          isRepairing ||
                          minecraftState === "preparing" ||
                          minecraftState === "running"
                        }
                      >
                        {isRepairing
                          ? "RÉPARATION..."
                          : "RÉPARER"}
                      </button>

                    </div>

                  </div>

                </div>

              )}

            </section>

          </div>

        </main>

      )}

      {/* ======================================================
          FOOTER
          ====================================================== */}

      <footer className="footer">

        <span>
          ETERNIA
        </span>

        <span>
          •
        </span>

        <span>
          MINECRAFT SERVER
        </span>

        <div className="background-dots">

          {backgrounds.map(
            (_, index) => (

              <button
                key={index}
                className={
                  index === background
                    ? "dot active"
                    : "dot"
                }
                onClick={() =>
                  setBackground(index)
                }
                aria-label={
                  `Fond ${index + 1}`
                }
              />

            ),
          )}

        </div>

        <span>
          v0.1.0
        </span>

      </footer>

    </div>
  );
}

// ============================================================
// EXPORT
// ============================================================

export default App;