import {
  useState,
  useEffect,
  useCallback,
  useRef,
  type ReactNode,
} from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import QRCode from "qrcode";
import sharexLogo from "./assets/sharex-logo.png";
import ConnectDeviceForm from "./ConnectDeviceForm";
import { version as appVersion } from "../package.json";
import {
  ArrowUp,
  ArrowUpRight,
  ArrowDown,
  ArrowDownUp,
  ArrowLeft,
  Check,
  CheckCheck,
  ChevronRight,
  ChevronDown,
  Copy,
  Download,
  Ellipsis,
  ExternalLink,
  File,
  FileAudio,
  FileImage,
  FileText,
  FileVideo,
  Folder,
  FolderOpen,
  FolderPlus,
  HardDrive,
  History,
  Info,
  LayoutGrid,
  List,
  LoaderCircle,
  LockKeyhole,
  Monitor,
  Moon,
  Network,
  Package,
  Play,
  Plus,
  Plug,
  RefreshCw,
  Search,
  Settings2,
  Shield,
  ShieldCheck,
  Smartphone,
  Square,
  Sun,
  Trash2,
  Upload,
  Wifi,
  X,
} from "lucide-react";
import {
  defaults,
  size,
  type Snapshot,
  type Settings,
  type FileItem,
  type Plugin,
} from "./types";

const empty: Snapshot = {
  settings: defaults,
  drives: [],
  running: false,
  started: 0,
  urls: [],
  devices: [],
  history: [],
  plugins: [],
};
type Page = "files" | "transfers" | "devices" | "plugins" | "settings";
function IconFile({ item }: { item: FileItem }) {
  const Icon = item.directory
    ? Folder
    : item.mime.startsWith("image/")
      ? FileImage
      : item.mime.startsWith("video/")
        ? FileVideo
        : item.mime.startsWith("audio/")
          ? FileAudio
          : item.mime.startsWith("text/") || item.mime.includes("pdf")
            ? FileText
            : File;
  return (
    <span className={`file-icon ${item.directory ? "folder-icon" : ""}`}>
      <Icon size={21} />
    </span>
  );
}
function Toggle({
  checked,
  onChange,
  label,
  disabled = false,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  disabled?: boolean;
}) {
  return (
    <button
      className="switch"
      type="button"
      role="switch"
      aria-label={label}
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
    >
      <span />
    </button>
  );
}
function Button({
  children,
  onClick,
  primary = false,
  disabled = false,
  className = "",
  title,
}: {
  children: ReactNode;
  onClick?: () => void;
  primary?: boolean;
  disabled?: boolean;
  className?: string;
  title?: string;
}) {
  return (
    <button
      title={title}
      className={`${primary ? "button-primary" : "button"} ${className}`}
      onClick={onClick}
      disabled={disabled}
    >
      {children}
    </button>
  );
}
function Modal({
  title,
  description,
  children,
  onClose,
  wide = false,
  error,
}: {
  title: string;
  description?: string;
  children: ReactNode;
  onClose: () => void;
  wide?: boolean;
  error?: string;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    ref.current?.showModal();
    const el = ref.current;
    return () => el?.close();
  }, []);
  return (
    <dialog
      ref={ref}
      className={`modal ${wide ? "wide" : ""}`}
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="modal-heading">
        <div>
          <h2>{title}</h2>
          {description && <p>{description}</p>}
        </div>
        <button
          className="icon-button"
          aria-label="Close dialog"
          onClick={onClose}
        >
          <X size={19} />
        </button>
      </div>
      {error && (
        <div className="error-banner" role="alert">
          {error}
        </div>
      )}
      {children}
    </dialog>
  );
}

export default function App() {
  const [data, setData] = useState<Snapshot>(empty),
    [page, setPage] = useState<Page>("files"),
    [files, setFiles] = useState<FileItem[]>([]),
    [location, setLocation] = useState(""),
    [query, setQuery] = useState(""),
    [view, setView] = useState("list"),
    [selected, setSelected] = useState<Set<string>>(new Set()),
    [loading, setLoading] = useState(false),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [notice, setNotice] = useState(""),
    [scope, setScope] = useState(false),
    [draft, setDraft] = useState<Settings>(defaults),
    [password, setPassword] = useState(""),
    [passwordChanged, setPasswordChanged] = useState(false),
    [qr, setQr] = useState(""),
    [selectedAddress, setSelectedAddress] = useState(""),
    [sort, setSort] = useState<"name" | "size" | "modified">("name"),
    [descending, setDescending] = useState(false),
    [filter, setFilter] = useState("all"),
    [catalog, setCatalog] = useState<Plugin[] | null>(null),
    [catalogError, setCatalogError] = useState(""),
    [trust, setTrust] = useState<{
      path?: string;
      package?: string;
      name: string;
    } | null>(null),
    [confirm, setConfirm] = useState<{
      title: string;
      description: string;
      run: () => Promise<void>;
    } | null>(null),
    [nameDialog, setNameDialog] = useState<{
      kind: "new" | "rename";
      name: string;
    } | null>(null),
    [destination, setDestination] = useState<{
      mode: string;
      path: string;
    } | null>(null),
    [destinationFolders, setDestinationFolders] = useState<FileItem[]>([]),
    [connection, setConnection] = useState(false),
    [preview, setPreview] = useState<{
      name: string;
      kind: string;
      content: string;
      location: string;
    } | null>(null),
    [now, setNow] = useState(Date.now());
  const native = isTauri();
  const connectionAddress = data.urls.includes(selectedAddress)
    ? selectedAddress
    : data.urls[0];
  const search = useRef<HTMLInputElement>(null);
  const refresh = useCallback(async () => {
    if (!native) return;
    const v = await invoke<Snapshot>("snapshot");
    setData(v);
    return v;
  }, [native]);
  const run = useCallback(async (action: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await action();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, []);
  useEffect(() => {
    if (!native) {
      setError(
        "Browser preview. Launch the desktop app to access drives and start sharing.",
      );
      return;
    }
    refresh().catch((e) => setError(String(e)));
    const id = setInterval(() => {
      refresh().catch(() => {});
    }, 1000);
    return () => clearInterval(id);
  }, [native, refresh]);
  useEffect(() => {
    setDraft(data.settings);
  }, [
    data.settings.rootMode,
    data.settings.rootPath,
    data.settings.privateMode,
  ]);
  useEffect(() => {
    document.documentElement.dataset.theme = data.settings.theme;
  }, [data.settings.theme]);
  useEffect(() => {
    if (!notice) return;
    const t = setTimeout(() => setNotice(""), 4000);
    return () => clearTimeout(t);
  }, [notice]);
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);
  useEffect(() => {
    if (connectionAddress)
      QRCode.toDataURL(connectionAddress, {
        width: 280,
        margin: 2,
        color: { dark: "#19171f", light: "#ffffff" },
      }).then(setQr);
    else setQr("");
  }, [connectionAddress]);
  const loadFiles = useCallback(async () => {
    if (!native) return;
    setLoading(true);
    try {
      const result = await invoke<{ items: FileItem[] }>("list_files", {
        location,
      });
      setFiles(result.items);
      setSelected(new Set());
      setError("");
    } catch (e) {
      setFiles([]);
      if (
        data.settings.rootPath ||
        data.settings.rootMode === "all" ||
        data.settings.privateMode
      )
        setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [
    native,
    location,
    data.settings.rootPath,
    data.settings.rootMode,
    data.settings.privateMode,
    data.settings.privateFiles.join("|"),
    data.settings.showHidden,
    data.drives.length,
  ]);
  useEffect(() => {
    loadFiles();
  }, [loadFiles]);
  useEffect(() => {
    function key(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "f") {
        e.preventDefault();
        search.current?.focus();
      }
      if (e.key === "Escape") {
        setSelected(new Set());
      }
      if (
        e.key === "Backspace" &&
        !(e.target instanceof HTMLInputElement) &&
        page === "files"
      ) {
        setLocation((v) => v.split("/").slice(0, -1).join("/"));
      }
    }
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [page]);
  useEffect(() => {
    if (!destination) return;
    invoke<{ items: FileItem[] }>("list_files", { location: destination.path })
      .then((v) => setDestinationFolders(v.items.filter((i) => i.directory)))
      .catch((e) => setError(String(e)));
  }, [destination?.path]);
  const save = async (next: Settings, pass?: string) => {
    const v = await invoke<Snapshot>("save_settings", {
      settings: next,
      password: pass ?? null,
    });
    setData(v);
    setDraft(v.settings);
    setPassword("");
    setPasswordChanged(false);
    setLocation("");
    await loadFiles();
    setNotice("Settings saved");
  };
  const chooseRoot = async () => {
    if (!native) return;
    const path = await open({
      directory: true,
      multiple: false,
      title: "Choose shared folder",
    });
    if (typeof path === "string")
      setDraft((v) => ({ ...v, rootPath: path, rootMode: "folder" }));
  };
  const privateFiles = async () => {
    const paths = await open({
      multiple: true,
      title: "Select files to share privately",
    });
    if (paths)
      setDraft((v) => ({
        ...v,
        privateFiles: [
          ...new Set([
            ...v.privateFiles,
            ...(Array.isArray(paths) ? paths : [paths]),
          ]),
        ],
      }));
  };
  const op = async (parameters: Record<string, string>) => {
    const result = await invoke<
      | string
      | { id: string; state: string; errors: string[]; completed: string[] }
    >("file_operation", { parameters });
    if (typeof result === "object" && result.id) {
      let job = result;
      while (["queued", "running"].includes(job.state)) {
        setNotice(
          `${parameters.mode === "move" ? "Moving" : "Copying"} files…`,
        );
        await new Promise((resolve) => setTimeout(resolve, 500));
        job = await invoke("operation_status", { id: result.id });
      }
      await loadFiles();
      if (job.state === "failed") throw new Error(job.errors.join("; "));
      setNotice(
        `${job.completed.length} items ${parameters.mode === "move" ? "moved" : "copied"}`,
      );
      return;
    }
    setNotice(typeof result === "string" ? result : "File operation started");
    await loadFiles();
  };
  const showPreview = async (item: FileItem) => {
    const result = await invoke<{
      name: string;
      kind: string;
      content: string;
    }>("read_preview", { location: item.location });
    setPreview({ ...result, location: item.location });
  };
  const showScope = () => {
    setDraft(data.settings);
    setScope(true);
  };
  const navigate = (path: string) => {
    setLocation(path);
    setQuery("");
    setSelected(new Set());
  };
  const copy = async (text: string) => {
    await navigator.clipboard.writeText(text);
    setNotice("Copied to clipboard");
  };
  const openPortal = async (path = "") => {
    if (!data.running) {
      setError("Start sharing to open the browser portal.");
      return;
    }
    await openUrl(`${connectionAddress}${path}`);
  };
  const choosePlugin = async () => {
    const path = await open({
      multiple: false,
      filters: [{ name: "ShareX plugin", extensions: ["zip"] }],
      title: "Install plugin ZIP",
    });
    if (typeof path === "string")
      setTrust({ path, name: path.split(/[\\/]/).pop() || "Plugin" });
  };
  const switchPage = (next: Page) => {
    setPage(next);
    setQuery("");
    setFilter("all");
    if (next === "settings") {
      setDraft(data.settings);
      setPassword("");
      setPasswordChanged(false);
    }
  };
  const visible = files
    .filter((f) => f.name.toLowerCase().includes(query.toLowerCase()))
    .sort(
      (a, b) =>
        Number(b.directory) - Number(a.directory) ||
        (descending ? -1 : 1) *
          (sort === "name" ? a.name.localeCompare(b.name) : a[sort] - b[sort]),
    );
  const pending = data.devices.filter(
    (d) =>
      d.status === "pending" || d.plugins.some((p) => p.startsWith("pending:")),
  );
  const active = data.history.filter((t) => t.status === "active");
  const received =
    data.totals?.received ??
    data.history
      .filter((t) => t.direction === "received")
      .reduce((n, t) => n + t.transferred, 0);
  const sent =
    data.totals?.sent ??
    data.history
      .filter((t) => t.direction === "sent")
      .reduce((n, t) => n + t.transferred, 0);
  const rootLabel = data.settings.privateMode
    ? "Selected files"
    : data.settings.rootMode === "all"
      ? "All eligible drives"
      : data.settings.rootPath || "No folder selected";
  const canModify = !data.settings.restrictModify && !data.settings.privateMode;
  const elapsed = data.running
    ? Math.max(0, Math.floor((now - data.started) / 1000))
    : 0;
  const navigation = [
    { id: "files" as Page, label: "Shared files", icon: FolderOpen },
    {
      id: "transfers" as Page,
      label: "Transfers",
      icon: ArrowDownUp,
      count: active.length,
    },
    {
      id: "devices" as Page,
      label: "Devices",
      icon: Monitor,
      count: pending.length,
    },
    { id: "plugins" as Page, label: "Plugins", icon: Plug },
  ];

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <a
          className="brand"
          aria-label="ShareX Desktop"
          href="#"
          onClick={(e) => {
            e.preventDefault();
            switchPage("files");
          }}
        >
          <img className="brand-logo" src={sharexLogo} alt="" />
          <span>
            ShareX<span className="desktop-label">DESKTOP</span>
          </span>
        </a>
        <div className="workspace-label">WORKSPACE</div>
        <nav aria-label="Main navigation">
          {navigation.map((n) => (
            <button
              key={n.id}
              aria-label={n.label}
              className={`nav-item ${page === n.id ? "active" : ""}`}
              onClick={() => switchPage(n.id)}
            >
              <n.icon size={19} />
              <span>{n.label}</span>
              {!!n.count && <span className="nav-count">{n.count}</span>}
            </button>
          ))}
        </nav>
        <div className="sidebar-divider" />
        <div className="sidebar-label">
          <span>SHARED STORAGE</span>
          <button
            className="icon-button"
            aria-label="Configure shared storage"
            onClick={showScope}
          >
            <Plus size={15} />
          </button>
        </div>
        <button
          className="storage-nav"
          onClick={() => {
            switchPage("files");
            navigate("");
          }}
        >
          <HardDrive size={18} />
          <div>
            <strong>
              {data.settings.privateMode
                ? "Private selection"
                : data.settings.rootMode === "all"
                  ? "This PC"
                  : data.settings.rootPath
                      .split(/[\\/]/)
                      .filter(Boolean)
                      .pop() || "Choose storage"}
            </strong>
            <small>
              {data.settings.privateMode
                ? `${data.settings.privateFiles.length} files`
                : data.settings.rootMode === "all"
                  ? `${data.drives.filter((d) => d.eligible).length} eligible drives`
                  : data.settings.rootPath
                    ? "Shared folder"
                    : "Set your sharing root"}
            </small>
          </div>
        </button>
        <div className="sidebar-bottom">
          <div className="local-note">
            <span className="local-note-icon">
              <ShieldCheck size={20} />
            </span>
            <strong>Your files stay yours.</strong>
            <p>
              Direct transfers on your network.
              <br />
              No cloud. No middleman.
            </p>
          </div>
          <button
            className={`nav-item ${page === "settings" ? "active" : ""}`}
            aria-label="Settings"
            onClick={() => switchPage("settings")}
          >
            <Settings2 size={19} />
            <span>Settings</span>
          </button>
          <div className="profile">
            <span className="computer-avatar">
              <Monitor size={18} />
            </span>
            <div>
              <strong>{data.settings.deviceName}</strong>
              <small>Windows desktop</small>
            </div>
            <span className={`status-dot ${data.running ? "online" : ""}`} />
          </div>
        </div>
      </aside>
      <div className="main-shell">
        <header className="topbar">
          <div className="breadcrumbs-top">
            Workspace <ChevronRight size={13} />
            <strong>
              {page === "files"
                ? "Shared files"
                : page[0].toUpperCase() + page.slice(1)}
            </strong>
          </div>
          <div className="topbar-actions">
            <span className="local-pill">
              <Wifi size={13} /> Local network
            </span>
            <button
              className="icon-button"
              title="Toggle appearance"
              aria-label="Toggle appearance"
              disabled={!native || data.running}
              onClick={() =>
                run(() =>
                  save({
                    ...data.settings,
                    theme: data.settings.theme === "dark" ? "light" : "dark",
                  }),
                )
              }
            >
              {data.settings.theme === "dark" ? (
                <Sun size={18} />
              ) : (
                <Moon size={18} />
              )}
            </button>
          </div>
        </header>
        <div className="body-shell">
          <main className="main-content">
            <div className="page-title">
              <div>
                <div className="eyebrow">YOUR NETWORK. YOUR FILES.</div>
                <h1>
                  {page === "files"
                    ? "Shared files"
                    : page === "transfers"
                      ? "Transfers"
                      : page === "devices"
                        ? "Devices"
                        : page === "plugins"
                          ? "Plugins"
                          : "Settings"}
                </h1>
                <p>
                  {page === "files"
                    ? "Browse and manage the files available on your network."
                    : page === "transfers"
                      ? "Every file, from departure to arrival."
                      : page === "devices"
                        ? "You decide who gets access."
                        : page === "plugins"
                          ? "More possibilities for your local network."
                          : "Make ShareX work your way."}
                </p>
              </div>
              {page === "files" ? (
                <Button disabled={!native || busy} onClick={showScope}>
                  <FolderPlus size={16} />
                  Manage storage
                </Button>
              ) : page === "transfers" ? (
                <Button
                  disabled={!native || !data.history.length}
                  onClick={() =>
                    setConfirm({
                      title: "Clear transfer history?",
                      description:
                        "Completed transfer records will be removed. Your files will remain on disk.",
                      run: async () => {
                        setData(await invoke("clear_history", { id: null }));
                      },
                    })
                  }
                >
                  <Trash2 size={16} />
                  Clear history
                </Button>
              ) : page === "plugins" ? (
                <Button
                  primary
                  disabled={!native || busy}
                  onClick={() => run(choosePlugin)}
                >
                  <Plus size={16} />
                  Install from ZIP
                </Button>
              ) : null}
            </div>
            {error && (
              <div className="error-banner" role="alert">
                <Info size={17} />
                <span>{error}</span>
                <button
                  className="icon-button"
                  aria-label="Dismiss error"
                  onClick={() => setError("")}
                >
                  <X size={15} />
                </button>
              </div>
            )}
            {page === "files" && (
              <>
                <section className="scope-strip">
                  <span className="scope-icon">
                    <HardDrive size={22} />
                  </span>
                  <div className="scope-copy">
                    <span>SHARING FROM</span>
                    <strong title={rootLabel}>{rootLabel}</strong>
                  </div>
                  <span className="subtle-badge">
                    <ShieldCheck size={13} />
                    {data.settings.privateMode
                      ? "Private"
                      : data.settings.restrictModify
                        ? "Read only"
                        : "Read & write"}
                  </span>
                  <button className="text-button" onClick={showScope}>
                    Change
                    <ChevronRight size={14} />
                  </button>
                </section>
                <div className="files-toolbar">
                  <div className="file-breadcrumbs">
                    <button
                      className="icon-button"
                      aria-label="Parent folder"
                      disabled={!location}
                      onClick={() =>
                        navigate(location.split("/").slice(0, -1).join("/"))
                      }
                    >
                      <ArrowLeft size={17} />
                    </button>
                    <button
                      onClick={() => navigate("")}
                      className="text-button"
                    >
                      <HardDrive size={15} />
                      {data.settings.rootMode === "all" ? "This PC" : "Storage"}
                    </button>
                    {location
                      .split("/")
                      .filter(Boolean)
                      .map((part, i) => (
                        <span key={i}>
                          <ChevronRight size={13} />
                          <button
                            onClick={() =>
                              navigate(
                                location
                                  .split("/")
                                  .slice(0, i + 1)
                                  .join("/"),
                              )
                            }
                          >
                            {part}
                          </button>
                        </span>
                      ))}
                  </div>
                  <div className="view-controls">
                    <button
                      className="icon-button"
                      title="Refresh files"
                      aria-label="Refresh files"
                      onClick={() => run(loadFiles)}
                      disabled={!native || loading}
                    >
                      <RefreshCw size={16} className={loading ? "spin" : ""} />
                    </button>
                    <span />
                    <button
                      className={`icon-button ${view === "list" ? "chosen" : ""}`}
                      aria-label="List view"
                      onClick={() => setView("list")}
                    >
                      <List size={17} />
                    </button>
                    <button
                      className={`icon-button ${view === "grid" ? "chosen" : ""}`}
                      aria-label="Grid view"
                      onClick={() => setView("grid")}
                    >
                      <LayoutGrid size={16} />
                    </button>
                  </div>
                </div>
                <div className="search-row">
                  <label className="search">
                    <Search size={16} />
                    <input
                      ref={search}
                      aria-label="Search files"
                      placeholder="Search in shared files…"
                      value={query}
                      onChange={(e) => setQuery(e.target.value)}
                    />
                    <kbd>Ctrl F</kbd>
                  </label>
                  {selected.size > 0 ? (
                    <span className="selection-count">
                      {selected.size} selected
                    </span>
                  ) : (
                    <span className="item-count">{files.length} items</span>
                  )}
                  <Button
                    disabled={
                      !native ||
                      !canModify ||
                      busy ||
                      (!data.settings.rootPath &&
                        data.settings.rootMode !== "all")
                    }
                    onClick={() => setNameDialog({ kind: "new", name: "" })}
                  >
                    <FolderPlus size={16} />
                    <span className="hide-small">New folder</span>
                  </Button>
                </div>
                {selected.size > 0 && (
                  <div className="selection-bar">
                    <strong>{selected.size} selected</strong>
                    <button
                      onClick={() =>
                        run(async () => {
                          if (!data.running)
                            throw new Error(
                              "Start sharing to download files through the browser.",
                            );
                          await openPortal(
                            `/ShareX?${new URLSearchParams({ action: "downloadFiles", data: JSON.stringify([...selected]), filename: "ShareX.zip" })}`,
                          );
                        })
                      }
                    >
                      <Download size={15} />
                      Download
                    </button>
                    <button
                      disabled={!canModify}
                      onClick={() => setDestination({ mode: "copy", path: "" })}
                    >
                      <Copy size={15} />
                      Copy
                    </button>
                    <button
                      disabled={!canModify}
                      onClick={() => setDestination({ mode: "move", path: "" })}
                    >
                      <ArrowUpRight size={15} />
                      Move
                    </button>
                    {selected.size === 1 && (
                      <button
                        disabled={!canModify}
                        onClick={() =>
                          setNameDialog({
                            kind: "rename",
                            name:
                              files.find((f) => selected.has(f.location))
                                ?.name || "",
                          })
                        }
                      >
                        Rename
                      </button>
                    )}
                    <button
                      className="danger-text"
                      disabled={!canModify}
                      onClick={() =>
                        setConfirm({
                          title: `Delete ${selected.size} ${selected.size === 1 ? "item" : "items"}?`,
                          description:
                            "This permanently deletes selected files and folders from your PC. This action cannot be undone.",
                          run: () =>
                            op({
                              action: "delFiles",
                              data: JSON.stringify([...selected]),
                            }),
                        })
                      }
                    >
                      <Trash2 size={15} />
                      Delete
                    </button>
                    <button
                      className="icon-button"
                      aria-label="Clear selection"
                      onClick={() => setSelected(new Set())}
                    >
                      <X size={15} />
                    </button>
                  </div>
                )}
                {loading ? (
                  <div className="empty-state">
                    <LoaderCircle className="spin" size={30} />
                    <p>Reading shared storage…</p>
                  </div>
                ) : !visible.length ? (
                  <div className="empty-state">
                    <div className="empty-folder">
                      <FolderOpen size={54} strokeWidth={1.1} />
                      <span>
                        <Plus size={17} />
                      </span>
                    </div>
                    <h2>
                      {query
                        ? "No matching files"
                        : data.settings.privateMode
                          ? "Your private selection"
                          : data.settings.rootPath ||
                              data.settings.rootMode === "all"
                            ? "Nothing here yet"
                            : "Your files, ready to connect."}
                    </h2>
                    <p>
                      {query
                        ? "Try another file name."
                        : data.settings.privateMode
                          ? "Only files you select are visible to connected browsers."
                          : data.settings.rootPath ||
                              data.settings.rootMode === "all"
                            ? "This shared location is empty."
                            : "Choose a folder, a drive, or all eligible drives to get started."}
                    </p>
                    {!query && (
                      <Button primary disabled={!native} onClick={showScope}>
                        <Plus size={16} />
                        {data.settings.privateMode
                          ? "Choose files"
                          : "Choose shared storage"}
                      </Button>
                    )}
                    <div className="empty-footnote">
                      <LockKeyhole size={12} />
                      BitLocker protected drives stay excluded.
                    </div>
                  </div>
                ) : view === "list" ? (
                  <div className="table-container">
                    <table className="file-table">
                      <thead>
                        <tr>
                          <th className="checkbox-cell">
                            <input
                              type="checkbox"
                              aria-label="Select all files"
                              checked={
                                visible.length > 0 &&
                                visible.every((f) => selected.has(f.location))
                              }
                              onChange={(e) =>
                                setSelected(
                                  e.target.checked
                                    ? new Set(visible.map((f) => f.location))
                                    : new Set(),
                                )
                              }
                            />
                          </th>
                          {(["name", "size", "modified"] as const).map(
                            (key) => (
                              <th key={key}>
                                <button
                                  onClick={() => {
                                    if (sort === key)
                                      setDescending(!descending);
                                    else {
                                      setSort(key);
                                      setDescending(false);
                                    }
                                  }}
                                >
                                  {key === "modified"
                                    ? "Date modified"
                                    : key[0].toUpperCase() + key.slice(1)}
                                  {sort === key && (
                                    <ChevronDown
                                      size={12}
                                      className={descending ? "rotate" : ""}
                                    />
                                  )}
                                </button>
                              </th>
                            ),
                          )}
                          <th className="row-end" />
                        </tr>
                      </thead>
                      <tbody>
                        {visible.map((item) => (
                          <tr
                            key={item.location}
                            className={
                              selected.has(item.location) ? "selected" : ""
                            }
                          >
                            <td className="checkbox-cell">
                              <input
                                type="checkbox"
                                aria-label={`Select ${item.name}`}
                                checked={selected.has(item.location)}
                                onChange={() =>
                                  setSelected((prev) => {
                                    const next = new Set(prev);
                                    next.has(item.location)
                                      ? next.delete(item.location)
                                      : next.add(item.location);
                                    return next;
                                  })
                                }
                              />
                            </td>
                            <td>
                              <button
                                className="file-name"
                                onClick={() =>
                                  item.directory
                                    ? navigate(item.location)
                                    : run(() => showPreview(item))
                                }
                              >
                                <IconFile item={item} />
                                <span>
                                  <strong>{item.name}</strong>
                                  <small>
                                    {item.directory
                                      ? "Folder"
                                      : item.mime.split("/")[1]?.toUpperCase()}
                                  </small>
                                </span>
                                {item.directory && (
                                  <ChevronRight
                                    className="row-chevron"
                                    size={14}
                                  />
                                )}
                              </button>
                            </td>
                            <td>{item.directory ? "—" : size(item.size)}</td>
                            <td>
                              {item.modified
                                ? new Date(item.modified).toLocaleDateString(
                                    undefined,
                                    {
                                      month: "short",
                                      day: "numeric",
                                      year: "numeric",
                                    },
                                  )
                                : "—"}
                            </td>
                            <td>
                              <button
                                className="icon-button row-action"
                                aria-label={`Show ${item.name} in Explorer`}
                                onClick={() =>
                                  run(async () => {
                                    await invoke("reveal_file", {
                                      location: item.location,
                                    });
                                  })
                                }
                              >
                                <ExternalLink size={15} />
                              </button>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                ) : (
                  <div className="file-grid">
                    {visible.map((item) => (
                      <div
                        className={`file-tile ${selected.has(item.location) ? "selected" : ""}`}
                        key={item.location}
                      >
                        <input
                          type="checkbox"
                          aria-label={`Select ${item.name}`}
                          checked={selected.has(item.location)}
                          onChange={() =>
                            setSelected((prev) => {
                              const n = new Set(prev);
                              n.has(item.location)
                                ? n.delete(item.location)
                                : n.add(item.location);
                              return n;
                            })
                          }
                        />
                        <button
                          onClick={() =>
                            item.directory
                              ? navigate(item.location)
                              : run(() => showPreview(item))
                          }
                        >
                          <IconFile item={item} />
                          <strong>{item.name}</strong>
                          <small>
                            {item.directory ? "Folder" : size(item.size)}
                          </small>
                        </button>
                      </div>
                    ))}
                  </div>
                )}
                <div className="workspace-footer">
                  <span>
                    {visible.length} {visible.length === 1 ? "item" : "items"}
                    {selected.size > 0 && ` · ${selected.size} selected`}
                  </span>
                  <span>
                    <ShieldCheck size={12} /> Shared on your terms
                  </span>
                </div>
              </>
            )}
            {page === "transfers" && (
              <>
                <div className="transfer-summary">
                  <div>
                    <span className="summary-icon">
                      <ArrowUpRight size={20} />
                    </span>
                    <div>
                      <small>Total sent</small>
                      <strong>{size(sent)}</strong>
                    </div>
                  </div>
                  <div>
                    <span className="summary-icon">
                      <ArrowDown size={20} />
                    </span>
                    <div>
                      <small>Total received</small>
                      <strong>{size(received)}</strong>
                    </div>
                  </div>
                  <div>
                    <span className="summary-icon">
                      <ArrowDownUp size={20} />
                    </span>
                    <div>
                      <small>In progress</small>
                      <strong>
                        {active.length}
                        <em> transfers</em>
                      </strong>
                    </div>
                  </div>
                </div>
                <div className="filter-tabs">
                  {["all", "sent", "received"].map((t) => (
                    <button
                      key={t}
                      className={filter === t ? "active" : ""}
                      onClick={() => setFilter(t)}
                    >
                      {t === "all"
                        ? "All transfers"
                        : t === "sent"
                          ? "Sent"
                          : "Received"}
                    </button>
                  ))}
                </div>
                {!data.history.length ? (
                  <Empty
                    icon={<ArrowDownUp size={33} />}
                    title="No transfers yet"
                    text="Files sent and received through your browser portal appear here."
                  />
                ) : (
                  <div className="record-list">
                    {data.history
                      .filter(
                        (t) =>
                          (filter === "all" || t.direction === filter) &&
                          t.name.toLowerCase().includes(query.toLowerCase()),
                      )
                      .map((t) => (
                        <div className="transfer-record" key={t.id}>
                          <span className="record-icon">
                            {t.direction === "sent" ? (
                              <ArrowUpRight size={20} />
                            ) : (
                              <ArrowDown size={20} />
                            )}
                          </span>
                          <div className="record-main">
                            <strong>{t.name}</strong>
                            <small>
                              {t.device} ·{" "}
                              {new Date(t.timestamp).toLocaleString()}
                            </small>
                            {t.status === "active" && (
                              <progress
                                aria-label={`Transfer ${t.name}`}
                                max={t.size || undefined}
                                value={t.size ? t.transferred : undefined}
                              />
                            )}
                          </div>
                          <div className="transfer-meta">
                            <strong>
                              {size(t.transferred)}
                              {t.size > 0 && t.status === "active"
                                ? ` / ${size(t.size)}`
                                : ""}
                            </strong>
                            <small>
                              {t.status === "active"
                                ? `${size(t.transferred / Math.max(0.1, t.elapsedMs / 1000))}/s`
                                : t.status}
                            </small>
                          </div>
                          <button
                            className="icon-button"
                            disabled={!t.path}
                            aria-label={`Reveal ${t.name}`}
                            onClick={() => run(() => revealItemInDir(t.path))}
                          >
                            <ExternalLink size={16} />
                          </button>
                        </div>
                      ))}
                  </div>
                )}
              </>
            )}
            {page === "devices" && (
              <>
                <div className="info-strip">
                  <ShieldCheck size={20} />
                  <div>
                    <strong>Access is always in your hands.</strong>
                    <p>
                      Approve new browsers here. Remember browsers you trust.
                      Revoke access at any time.
                    </p>
                  </div>
                </div>
                <div className="section-header">
                  <h2>Browser access</h2>
                  <span>{data.devices.length} browsers</span>
                </div>
                {!data.devices.length ? (
                  <Empty
                    icon={<Monitor size={34} />}
                    title="A connection starts here"
                    text="Start sharing, then open your connection address on another device. Browser requests will appear here."
                  />
                ) : (
                  <div className="record-list">
                    {data.devices.map((d) => (
                      <div className="device-record" key={d.id}>
                        <span className="record-icon">
                          {d.name.includes("Android") ||
                          d.name.includes("iPhone") ? (
                            <Smartphone size={23} />
                          ) : (
                            <Monitor size={23} />
                          )}
                        </span>
                        <div className="record-main">
                          <strong>{d.name}</strong>
                          <small>
                            {d.ip} ·{" "}
                            {d.remembered ? "Remembered browser" : d.status}
                          </small>
                          {d.plugins
                            .filter((p) => p.startsWith("pending:"))
                            .map((p) => (
                              <button
                                className="text-button"
                                key={p}
                                onClick={() =>
                                  run(async () =>
                                    setData(
                                      await invoke("device_action", {
                                        id: d.id,
                                        action: "plugin",
                                        remember: false,
                                        package: p.slice(8),
                                      }),
                                    ),
                                  )
                                }
                              >
                                Approve plugin: {p.slice(8)}
                                <ChevronRight size={14} />
                              </button>
                            ))}
                        </div>
                        {d.status === "pending" ? (
                          <div className="device-actions">
                            <Button
                              onClick={() =>
                                run(async () =>
                                  setData(
                                    await invoke("device_action", {
                                      id: d.id,
                                      action: "approve",
                                      remember: true,
                                      package: null,
                                    }),
                                  ),
                                )
                              }
                            >
                              <CheckCheck size={15} />
                              Remember
                            </Button>
                            <Button
                              primary
                              onClick={() =>
                                run(async () =>
                                  setData(
                                    await invoke("device_action", {
                                      id: d.id,
                                      action: "approve",
                                      remember: false,
                                      package: null,
                                    }),
                                  ),
                                )
                              }
                            >
                              <Check size={15} />
                              Approve
                            </Button>
                          </div>
                        ) : (
                          <span
                            className={`subtle-badge ${d.status === "approved" ? "good" : ""}`}
                          >
                            {d.status === "approved" ? (
                              <Check size={12} />
                            ) : (
                              <LockKeyhole size={12} />
                            )}{" "}
                            {d.status}
                          </span>
                        )}
                        <button
                          className="icon-button danger-text"
                          aria-label={`Revoke ${d.name}`}
                          onClick={() =>
                            run(async () =>
                              setData(
                                await invoke("device_action", {
                                  id: d.id,
                                  action: "revoke",
                                  remember: false,
                                  package: null,
                                }),
                              ),
                            )
                          }
                        >
                          <X size={17} />
                        </button>
                        {d.status === "denied" && (
                          <button
                            className="text-button"
                            onClick={() =>
                              run(async () =>
                                setData(
                                  await invoke("device_action", {
                                    id: d.id,
                                    action: "forget",
                                    remember: false,
                                    package: null,
                                  }),
                                ),
                              )
                            }
                          >
                            Forget
                          </button>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </>
            )}
            {page === "plugins" && (
              <>
                <div className="filter-tabs">
                  <button
                    className={!catalog ? "active" : ""}
                    onClick={() => setCatalog(null)}
                  >
                    Installed <span>{data.plugins.length}</span>
                  </button>
                  <button
                    className={catalog ? "active" : ""}
                    onClick={() =>
                      run(async () => {
                        setCatalogError("");
                        try {
                          setCatalog(await invoke("plugin_catalog"));
                        } catch (e) {
                          setCatalog([]);
                          setCatalogError(String(e));
                        }
                      })
                    }
                  >
                    Plugin store
                    <ArrowUpRight size={13} />
                  </button>
                </div>
                {catalogError && (
                  <div className="error-banner" role="alert">
                    {catalogError}
                  </div>
                )}
                {(catalog || data.plugins).length === 0 ? (
                  <Empty
                    icon={<Plug size={34} />}
                    title={
                      catalog
                        ? "No store results"
                        : "Small tools. More possibilities."
                    }
                    text={
                      catalog
                        ? "Check your internet connection, then try again."
                        : "Install a ShareX plugin from the store or a ZIP file to extend your browser portal."
                    }
                  />
                ) : (
                  <div className="plugin-list">
                    {(catalog || data.plugins).map((p) => (
                      <article className="plugin-entry" key={p.package}>
                        <span className="plugin-icon">
                          <Package size={25} />
                        </span>
                        <div>
                          <h3>
                            {p.name}
                            <span>v{p.version}</span>
                          </h3>
                          <p>{p.description}</p>
                          <small>{p.author}</small>
                        </div>
                        {catalog ? (
                          <Button
                            disabled={
                              data.plugins.some(
                                (x) =>
                                  x.package === p.package &&
                                  x.version === p.version,
                              ) || busy
                            }
                            onClick={() =>
                              setTrust({ package: p.package, name: p.name })
                            }
                          >
                            {data.plugins.some(
                              (x) =>
                                x.package === p.package &&
                                x.version === p.version,
                            )
                              ? "Installed"
                              : data.plugins.some(
                                    (x) => x.package === p.package,
                                  )
                                ? "Update"
                                : "Install"}
                            <Download size={14} />
                          </Button>
                        ) : (
                          <>
                            <Toggle
                              label={`Enable ${p.name}`}
                              checked={p.enabled}
                              onChange={() =>
                                run(async () =>
                                  setData(
                                    await invoke("plugin_action", {
                                      uid: p.uid,
                                      action: "toggle",
                                    }),
                                  ),
                                )
                              }
                            />
                            <button
                              className="icon-button"
                              aria-label={`Open ${p.name}`}
                              disabled={!p.enabled || !data.running}
                              onClick={() =>
                                run(() => openPortal(`/SharexApp/${p.uid}/`))
                              }
                            >
                              <ExternalLink size={16} />
                            </button>
                            <button
                              className="icon-button danger-text"
                              aria-label={`Uninstall ${p.name}`}
                              onClick={() =>
                                setConfirm({
                                  title: `Uninstall ${p.name}?`,
                                  description:
                                    "Plugin files will be removed. Saved plugin data will be kept for a later installation.",
                                  run: async () => {
                                    setData(
                                      await invoke("plugin_action", {
                                        uid: p.uid,
                                        action: "uninstall",
                                      }),
                                    );
                                  },
                                })
                              }
                            >
                              <Trash2 size={16} />
                            </button>
                          </>
                        )}
                      </article>
                    ))}
                  </div>
                )}
              </>
            )}
            {page === "settings" && (
              <>
                <div className="settings-form">
                  {data.running && (
                    <div className="info-strip">
                      <Info size={19} />
                      <div>
                        <strong>Stop sharing to edit settings.</strong>
                        <p>
                          Current connections will close when sharing stops.
                        </p>
                      </div>
                    </div>
                  )}
                  <fieldset disabled={data.running || !native || busy}>
                    <section>
                      <h2>General</h2>
                      <label className="setting-field">
                        <span>
                          Device name<small>Shown to connected browsers.</small>
                        </span>
                        <input
                          value={draft.deviceName}
                          maxLength={80}
                          onChange={(e) =>
                            setDraft({ ...draft, deviceName: e.target.value })
                          }
                        />
                      </label>
                      <label className="setting-field">
                        <span>Appearance</span>
                        <select
                          value={draft.theme}
                          onChange={(e) =>
                            setDraft({ ...draft, theme: e.target.value })
                          }
                        >
                          <option value="dark">Dark</option>
                          <option value="light">Light</option>
                        </select>
                      </label>
                      <label className="setting-field">
                        <span>Browser theme</span>
                        <select
                          value={draft.portalTheme}
                          onChange={(e) =>
                            setDraft({ ...draft, portalTheme: e.target.value })
                          }
                        >
                          {[
                            ["default", "Default"],
                            ["zen", "Zen Inspired"],
                            ["2077", "2077"],
                            ["hex", "hex"],
                            ["purple-rain", "Purple Rain"],
                            ["light-green", "Light Green"],
                            ["claude-plus", "Claude +"],
                          ].map(([v, l]) => (
                            <option value={v} key={v}>
                              {l}
                            </option>
                          ))}
                        </select>
                      </label>
                    </section>
                    <section>
                      <h2>Network & access</h2>
                      <label className="setting-field">
                        <span>
                          Sharing port<small>Plugins use the next port.</small>
                        </span>
                        <input
                          type="number"
                          min={1024}
                          max={65534}
                          value={draft.port}
                          onChange={(e) =>
                            setDraft({ ...draft, port: Number(e.target.value) })
                          }
                        />
                      </label>
                      <SettingRow
                        title="Require browser approval"
                        description="Review each new browser before it can access files."
                      >
                        <Toggle
                          checked={draft.requireApproval}
                          label="Require browser approval"
                          onChange={(v) =>
                            setDraft({ ...draft, requireApproval: v })
                          }
                        />
                      </SettingRow>
                      <SettingRow
                        title="HTTPS"
                        description="Encrypt connections with a local certificate. Browsers will ask you to trust it."
                      >
                        <Toggle
                          checked={draft.https}
                          label="HTTPS"
                          onChange={(v) => setDraft({ ...draft, https: v })}
                        />
                      </SettingRow>
                      <label className="setting-field">
                        <span>
                          Browser password
                          <small>
                            {draft.passwordSet
                              ? "Password is set. Enter a replacement, or clear it below."
                              : "Optional. Password grants browser access."}
                          </small>
                        </span>
                        <input
                          type="password"
                          autoComplete="new-password"
                          maxLength={128}
                          placeholder={
                            draft.passwordSet
                              ? "••••••••"
                              : "At least 8 characters"
                          }
                          value={password}
                          onChange={(e) => {
                            setPassword(e.target.value);
                            setPasswordChanged(true);
                          }}
                        />
                      </label>
                      {draft.passwordSet && (
                        <button
                          className="text-button danger-text"
                          onClick={() => {
                            setPassword("");
                            setPasswordChanged(true);
                            setNotice(
                              "Password will be removed when you save.",
                            );
                          }}
                        >
                          Remove password on save
                        </button>
                      )}
                      <label className="setting-field">
                        <span>Session duration</span>
                        <select
                          value={draft.sessionHours}
                          onChange={(e) =>
                            setDraft({
                              ...draft,
                              sessionHours: Number(e.target.value),
                            })
                          }
                        >
                          {[1, 6, 12, 24, 48, 168].map((n) => (
                            <option key={n} value={n}>
                              {n} hours
                            </option>
                          ))}
                        </select>
                      </label>
                    </section>
                    <section>
                      <h2>File access</h2>
                      <SettingRow
                        title="Read-only access"
                        description="Block uploads, deletion, renaming, copying, and moving in shared storage."
                      >
                        <Toggle
                          checked={draft.restrictModify}
                          label="Read-only access"
                          onChange={(v) =>
                            setDraft({ ...draft, restrictModify: v })
                          }
                        />
                      </SettingRow>
                      <SettingRow
                        title="Show hidden files"
                        description="Include hidden and system files inside your shared root."
                      >
                        <Toggle
                          checked={draft.showHidden}
                          label="Show hidden files"
                          onChange={(v) =>
                            setDraft({ ...draft, showHidden: v })
                          }
                        />
                      </SettingRow>
                    </section>
                    <section>
                      <h2>Plugin development</h2>
                      <SettingRow
                        title="Development connection"
                        description="Allow your local plugin project to connect using a revocable key."
                      >
                        <Toggle
                          checked={draft.pluginDevelopment}
                          label="Plugin development"
                          onChange={(v) =>
                            setDraft({ ...draft, pluginDevelopment: v })
                          }
                        />
                      </SettingRow>
                      {data.settings.devToken && (
                        <div className="dev-controls">
                          <Button
                            onClick={() =>
                              run(() =>
                                copy(
                                  `${connectionAddress || `${data.settings.https ? "https" : "http"}://127.0.0.1:${draft.port}`}#sharex-dev-token=${data.settings.devToken}`,
                                ),
                              )
                            }
                          >
                            <Copy size={14} />
                            Copy connection
                          </Button>
                          <Button
                            onClick={() =>
                              run(async () =>
                                setData(await invoke("reset_development_key")),
                              )
                            }
                          >
                            <RefreshCw size={14} />
                            Reset key
                          </Button>
                        </div>
                      )}
                    </section>
                    <div className="save-bar">
                      <span>Changes apply to your next sharing session.</span>
                      <Button
                        primary
                        onClick={() =>
                          run(() =>
                            save(draft, passwordChanged ? password : undefined),
                          )
                        }
                      >
                        <Check size={16} />
                        Save changes
                      </Button>
                    </div>
                  </fieldset>
                  <div className="about">
                    <span className="brand-word">
                      ShareX Desktop <small>{appVersion}</small>
                    </span>
                    <p>
                      Built with Tauri v2. Browser portal by Akansh Sirohi and
                      ShareX contributors.
                      <br />
                      AGPL-3.0-or-later · Desktop edition of ShareX Android.
                    </p>
                  </div>
                </div>
              </>
            )}
          </main>
          <aside className="inspector">
            <div className="inspector-heading">
              <span>SHARING SESSION</span>
              <span className={`live-label ${data.running ? "is-live" : ""}`}>
                <span
                  className={`status-dot ${data.running ? "online" : ""}`}
                />
                {data.running ? "Live" : "Offline"}
              </span>
            </div>
            <div className={`connection-orb ${data.running ? "running" : ""}`}>
              <div />
              <div />
              <span>
                <Wifi size={32} strokeWidth={1.5} />
              </span>
            </div>
            <h2>
              {data.running ? "You’re connected." : "Ready when you are."}
            </h2>
            <p className="inspector-description">
              {data.running
                ? "Your files are available to approved browsers on this network."
                : "Start a session to share files with any browser on your network."}
            </p>
            <Button
              primary
              className={`share-button ${data.running ? "stop-button" : ""}`}
              disabled={!native || busy}
              onClick={() =>
                run(async () => {
                  setData(
                    await invoke("set_sharing", { enabled: !data.running }),
                  );
                  setNotice(
                    data.running ? "Sharing stopped" : "Sharing started",
                  );
                })
              }
            >
              {busy ? (
                <LoaderCircle size={17} className="spin" />
              ) : data.running ? (
                <Square size={14} fill="currentColor" />
              ) : (
                <Play size={16} fill="currentColor" />
              )}
              {data.running ? "Stop sharing" : "Start sharing"}
            </Button>
            <div className="inspector-divider" />
            <div className="inspector-section-title">
              CONNECTION ADDRESS
              {data.running && <span className="status-dot online" />}
            </div>
            {data.running ? (
              <>
                <div className="address-box">
                  <code>{connectionAddress}</code>
                  <button
                    className="icon-button"
                    aria-label="Copy connection address"
                    onClick={() => run(() => copy(connectionAddress))}
                  >
                    <Copy size={15} />
                  </button>
                </div>
                {data.urls.length > 1 && (
                  <select
                    className="address-select"
                    aria-label="Other network interfaces"
                    value={connectionAddress}
                    onChange={(e) => setSelectedAddress(e.target.value)}
                  >
                    {data.urls.map((url) => (
                      <option key={url}>{url}</option>
                    ))}
                  </select>
                )}
                <button
                  className="portal-link"
                  onClick={() => run(() => openPortal())}
                >
                  Open browser portal
                  <ArrowUpRight size={14} />
                </button>
                {qr && (
                  <div className="qr-section">
                    <img
                      src={qr}
                      width={132}
                      height={132}
                      alt={`QR code for ${connectionAddress}`}
                    />
                    <span>Scan. Connect. Share.</span>
                    <small>Same Wi-Fi. Any device.</small>
                  </div>
                )}
              </>
            ) : (
              <div className="address-placeholder">
                <Network size={17} />
                <span>Available when sharing starts</span>
              </div>
            )}
            <div className="session-details">
              <div>
                <span>
                  <Monitor size={14} />
                  Approved browsers
                </span>
                <strong>
                  {data.devices.filter((d) => d.status === "approved").length}
                </strong>
              </div>
              <div>
                <span>
                  <History size={14} />
                  Session time
                </span>
                <strong>
                  {data.running
                    ? `${String(Math.floor(elapsed / 3600)).padStart(2, "0")}:${String(Math.floor(elapsed / 60) % 60).padStart(2, "0")}:${String(elapsed % 60).padStart(2, "0")}`
                    : "—"}
                </strong>
              </div>
              <div>
                <span>
                  <Shield size={14} />
                  Access
                </span>
                <strong>
                  {data.settings.requireApproval
                    ? "Approval required"
                    : data.settings.passwordSet
                      ? "Password"
                      : "Open"}
                </strong>
              </div>
            </div>
            {pending.length > 0 && (
              <button
                className="pending-alert"
                onClick={() => switchPage("devices")}
              >
                <span className="status-dot online" />
                {pending.length} access{" "}
                {pending.length === 1 ? "request" : "requests"}
                <ChevronRight size={15} />
              </button>
            )}
            <div className="connect-other">
              <div>
                <Smartphone size={18} />
                <strong>Another ShareX device?</strong>
              </div>
              <p>Open its connection address to send and receive files.</p>
              <button
                className="text-button"
                disabled={!native}
                onClick={() => {
                  setError("");
                  setConnection(true);
                }}
              >
                Connect to a device
                <ArrowUpRight size={14} />
              </button>
            </div>
            <div className="inspector-bottom">
              <LockKeyhole size={13} />
              <span>Local by design.</span>
              <span className="version">v{appVersion}</span>
            </div>
          </aside>
        </div>
      </div>
      {notice && (
        <div className="toast" role="status">
          <Check size={17} />
          {notice}
        </div>
      )}
      {scope && (
        <Modal
          error={error}
          title="Choose what you share"
          description="Your sharing root controls what connected browsers can see."
          onClose={() => setScope(false)}
          wide
        >
          <div className="scope-options">
            {[
              {
                id: "all",
                icon: Monitor,
                title: "All drives",
                text: "Every eligible drive",
              },
              {
                id: "drive",
                icon: HardDrive,
                title: "One drive",
                text: "A single disk",
              },
              {
                id: "folder",
                icon: Folder,
                title: "One folder",
                text: "Only what you choose",
              },
            ].map((o) => (
              <button
                disabled={data.running}
                key={o.id}
                className={
                  draft.rootMode === o.id && !draft.privateMode ? "active" : ""
                }
                onClick={() =>
                  setDraft({
                    ...draft,
                    privateMode: false,
                    rootMode: o.id as Settings["rootMode"],
                    rootPath: o.id === "folder" ? "" : draft.rootPath,
                  })
                }
              >
                <o.icon size={25} />
                <strong>{o.title}</strong>
                <small>{o.text}</small>
                {draft.rootMode === o.id && !draft.privateMode && (
                  <Check size={15} />
                )}
              </button>
            ))}
          </div>
          <div className="protection-note">
            <ShieldCheck size={18} />
            <p>
              BitLocker protected drives are always excluded. Drives with
              unknown protection status and linked folders are excluded too.
            </p>
          </div>
          {data.running && (
            <div className="error-banner">
              Stop sharing before changing shared storage.
            </div>
          )}
          <fieldset disabled={data.running || busy}>
            <SettingRow
              title="Private selection"
              description="Share selected files only. Incoming files go to ShareX’s Received folder."
            >
              <Toggle
                checked={draft.privateMode}
                label="Private selection"
                onChange={(v) => setDraft({ ...draft, privateMode: v })}
              />
            </SettingRow>
            {draft.privateMode ? (
              <div className="private-selection">
                <Button onClick={() => run(privateFiles)}>
                  <Plus size={15} />
                  Add files
                </Button>
                <span>{draft.privateFiles.length} files selected</span>
                <div>
                  {draft.privateFiles.map((p, i) => (
                    <div className="private-file" key={p}>
                      <File size={14} />
                      <span title={p}>{p.split(/[\\/]/).pop()}</span>
                      <button
                        className="icon-button"
                        aria-label={`Remove ${p}`}
                        onClick={() =>
                          setDraft({
                            ...draft,
                            privateFiles: draft.privateFiles.filter(
                              (_, n) => n !== i,
                            ),
                          })
                        }
                      >
                        <X size={14} />
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            ) : draft.rootMode === "folder" ? (
              <div className="folder-picker">
                <FolderOpen size={22} />
                <div>
                  <strong>{draft.rootPath || "No folder selected"}</strong>
                  <small>Everything inside this folder will be shared.</small>
                </div>
                <Button onClick={() => run(chooseRoot)}>Browse…</Button>
              </div>
            ) : (
              <div className="drive-picker">
                {data.drives.map((d) => (
                  <button
                    key={d.path}
                    disabled={!d.eligible || draft.rootMode === "all"}
                    className={
                      (draft.rootMode === "all" && d.eligible) ||
                      draft.rootPath === d.path
                        ? "active"
                        : ""
                    }
                    onClick={() => setDraft({ ...draft, rootPath: d.path })}
                  >
                    <HardDrive size={23} />
                    <div>
                      <strong>
                        {d.name || "Local disk"} <span>{d.path}</span>
                      </strong>
                      <small>
                        {d.eligible
                          ? `${size(d.free)} free of ${size(d.total)}`
                          : d.protection}
                      </small>
                    </div>
                    {!d.eligible ? (
                      <LockKeyhole size={17} />
                    ) : draft.rootMode === "all" ||
                      draft.rootPath === d.path ? (
                      <Check size={17} />
                    ) : null}
                  </button>
                ))}
                {!data.drives.length && (
                  <p>No drives detected. Refresh to check drive protection.</p>
                )}
                <button
                  className="text-button"
                  onClick={() =>
                    run(async () => setData(await invoke("refresh_drives")))
                  }
                >
                  <RefreshCw size={14} />
                  Refresh drives
                </button>
              </div>
            )}
            <div className="modal-footer">
              <Button onClick={() => setScope(false)}>Cancel</Button>
              <Button
                primary
                disabled={
                  busy ||
                  (!draft.privateMode &&
                    draft.rootMode !== "all" &&
                    !draft.rootPath)
                }
                onClick={() =>
                  run(async () => {
                    await save(draft);
                    setScope(false);
                  })
                }
              >
                <Check size={16} />
                Use this storage
              </Button>
            </div>
          </fieldset>
        </Modal>
      )}
      {nameDialog && (
        <Modal
          error={error}
          title={nameDialog.kind === "new" ? "New folder" : "Rename item"}
          onClose={() => setNameDialog(null)}
        >
          <form
            onSubmit={(e) => {
              e.preventDefault();
              run(async () => {
                await op(
                  nameDialog.kind === "new"
                    ? {
                        action: "newF",
                        parent: location,
                        name: nameDialog.name,
                      }
                    : {
                        action: "renF",
                        old_n: [...selected][0],
                        new_n: nameDialog.name,
                        parent: location,
                      },
                );
                setNameDialog(null);
              });
            }}
          >
            <label className="input-label">
              Name
              <input
                autoFocus
                value={nameDialog.name}
                onChange={(e) =>
                  setNameDialog({ ...nameDialog, name: e.target.value })
                }
                required
                maxLength={240}
              />
            </label>
            <div className="modal-footer">
              <Button onClick={() => setNameDialog(null)}>Cancel</Button>
              <button
                type="submit"
                className="button-primary"
                disabled={busy || !nameDialog.name.trim()}
              >
                Save
              </button>
            </div>
          </form>
        </Modal>
      )}
      {destination && (
        <Modal
          error={error}
          title={`${destination.mode === "copy" ? "Copy" : "Move"} ${selected.size} items`}
          description="Choose a destination. Existing files will not be overwritten."
          onClose={() => setDestination(null)}
        >
          <div className="destination-path">
            <button
              className="icon-button"
              aria-label="Parent destination"
              disabled={!destination.path}
              onClick={() =>
                setDestination({
                  ...destination,
                  path: destination.path.split("/").slice(0, -1).join("/"),
                })
              }
            >
              <ArrowUp size={16} />
            </button>
            <span>{destination.path || "Storage"}</span>
          </div>
          <div className="destination-folders">
            {destinationFolders.map((f) => (
              <button
                key={f.location}
                onClick={() =>
                  setDestination({ ...destination, path: f.location })
                }
              >
                <Folder size={18} />
                {f.name}
                <ChevronRight size={15} />
              </button>
            ))}
            {!destinationFolders.length && <p>No subfolders</p>}
          </div>
          <div className="modal-footer">
            <Button onClick={() => setDestination(null)}>Cancel</Button>
            <Button
              primary
              disabled={busy}
              onClick={() =>
                run(async () => {
                  await op({
                    action: "transferFiles",
                    data: JSON.stringify([...selected]),
                    mode: destination.mode,
                    destination: destination.path,
                  });
                  setDestination(null);
                })
              }
            >
              {destination.mode === "copy" ? "Copy" : "Move"} here
            </Button>
          </div>
        </Modal>
      )}
      {trust && (
        <Modal
          error={error}
          title={`Install ${trust.name}?`}
          description="Plugins run code in your browser and can access files through your approved session. Install only plugins you trust."
          onClose={() => setTrust(null)}
        >
          <div className="protection-note">
            <Shield size={22} />
            <p>
              Review the author and source before installing. Each browser must
              receive plugin approval before opening it.
            </p>
          </div>
          <div className="modal-footer">
            <Button onClick={() => setTrust(null)}>Cancel</Button>
            <Button
              primary
              disabled={busy}
              onClick={() =>
                run(async () => {
                  setData(
                    await invoke("install_plugin", {
                      path: trust.path ?? null,
                      package: trust.package ?? null,
                    }),
                  );
                  setTrust(null);
                  setNotice("Plugin installed");
                })
              }
            >
              {busy ? (
                <LoaderCircle size={16} className="spin" />
              ) : (
                <Download size={16} />
              )}
              Trust and install
            </Button>
          </div>
        </Modal>
      )}
      {confirm && (
        <Modal
          error={error}
          title={confirm.title}
          description={confirm.description}
          onClose={() => setConfirm(null)}
        >
          <div className="modal-footer">
            <Button onClick={() => setConfirm(null)}>Cancel</Button>
            <Button
              className="danger-button"
              disabled={busy}
              onClick={() =>
                run(async () => {
                  await confirm.run();
                  setConfirm(null);
                })
              }
            >
              Confirm
            </Button>
          </div>
        </Modal>
      )}
      {preview && (
        <Modal
          error={error}
          title={preview.name}
          onClose={() => setPreview(null)}
          wide
        >
          {preview.kind === "image" ? (
            <img
              className="image-preview"
              src={preview.content}
              alt={preview.name}
            />
          ) : preview.kind === "text" ? (
            <pre className="text-preview">{preview.content}</pre>
          ) : (
            <div className="empty-state preview-empty">
              <File size={38} />
              <h2>Open in the browser portal</h2>
              <p>
                The browser portal supports video, audio, PDF, Office documents,
                and other file previews.
              </p>
              <Button
                primary
                disabled={!data.running}
                onClick={() =>
                  run(() =>
                    openPortal(`/#${new URLSearchParams({ location })}`),
                  )
                }
              >
                <ExternalLink size={15} />
                Open browser portal
              </Button>
              {!data.running && <small>Start sharing first.</small>}
            </div>
          )}
          <div className="modal-footer">
            <Button
              onClick={() =>
                run(async () => {
                  await invoke("reveal_file", { location: preview.location });
                })
              }
            >
              <ExternalLink size={15} />
              Show in Explorer
            </Button>
            <Button onClick={() => setPreview(null)}>Close</Button>
          </div>
        </Modal>
      )}
      {connection && (
        <Modal
          title="Connect to a ShareX device"
          description="Use the protocol, IP address, and port shown in ShareX on the other device. Both devices must be on the same network."
          onClose={() => setConnection(false)}
        >
          <ConnectDeviceForm
            onNotice={setNotice}
            onOpened={() => {
              setConnection(false);
              setNotice("Opening device in your browser");
            }}
          />
        </Modal>
      )}
    </div>
  );
}
function SettingRow({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <div className="setting-row">
      <div>
        <strong>{title}</strong>
        <p>{description}</p>
      </div>
      {children}
    </div>
  );
}
function Empty({
  icon,
  title,
  text,
}: {
  icon: ReactNode;
  title: string;
  text: string;
}) {
  return (
    <div className="empty-state generic-empty">
      <div className="empty-symbol">{icon}</div>
      <h2>{title}</h2>
      <p>{text}</p>
    </div>
  );
}
