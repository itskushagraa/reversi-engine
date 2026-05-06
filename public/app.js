const { createElement: h, useEffect, useMemo, useState } = React;

const FILES = ["a", "b", "c", "d", "e", "f", "g", "h"];
const DEPTHS = [3, 5, 7, 9, 11];
const SIDES = ["Black", "White"];

function coordFor(index) {
  return `${FILES[index % 8]}${Math.floor(index / 8) + 1}`;
}

function App() {
  const [depth, setDepth] = useState(7);
  const [humanPlayer, setHumanPlayer] = useState("Black");
  const [history, setHistory] = useState([]);
  const [cursor, setCursor] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [depthOpen, setDepthOpen] = useState(false);
  const [newGamePulse, setNewGamePulse] = useState(false);

  const game = history[cursor] ?? null;
  const liveGame = history[history.length - 1] ?? null;
  const isViewingPast = history.length > 0 && cursor < history.length - 1;
  const canGoBack = cursor > 0;
  const canGoForward = cursor < history.length - 1;

  useEffect(() => {
    newGame(depth, humanPlayer);
  }, []);

  useEffect(() => {
    renderLucideIcons();
  });

  useEffect(() => {
    function onKeyDown(event) {
      if (event.key === "ArrowLeft") {
        event.preventDefault();
        setCursor((value) => Math.max(0, value - 1));
      }
      if (event.key === "ArrowRight") {
        event.preventDefault();
        setCursor((value) => Math.min(history.length - 1, value + 1));
      }
      if (event.key === "Escape") {
        setDepthOpen(false);
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [history.length]);

  const legalSet = useMemo(() => new Set(game?.legalMoves ?? []), [game]);
  const canPlay =
    game &&
    !busy &&
    !isViewingPast &&
    !game.gameOver &&
    game.currentPlayer === humanPlayer;

  async function newGame(nextDepth = depth, nextHumanPlayer = humanPlayer) {
    setBusy(true);
    setError("");
    setDepthOpen(false);
    setNewGamePulse(true);
    window.setTimeout(() => setNewGamePulse(false), 900);
    try {
      const response = await fetch(`/api/new?depth=${nextDepth}&human=${nextHumanPlayer}`);
      const data = await response.json();
      setHistory([data]);
      setCursor(0);
    } catch (err) {
      setError(`Could not start game: ${err.message}`);
    } finally {
      setBusy(false);
    }
  }

  async function playMove(coord) {
    if (!canPlay || !legalSet.has(coord)) {
      return;
    }

    setBusy(true);
    setError("");
    try {
      const response = await fetch("/api/move", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          black: game.black,
          white: game.white,
          currentPlayer: game.currentPlayer,
          humanPlayer,
          move: coord,
          depth,
        }),
      });
      const data = await response.json();
      if (!response.ok) {
        throw new Error(data.error || "Move failed");
      }
      setHistory((items) => {
        const next = items.slice(0, cursor + 1).concat(data);
        setCursor(next.length - 1);
        return next;
      });
    } catch (err) {
      setError(err.message);
    } finally {
      setBusy(false);
    }
  }

  function chooseSide(side) {
    setHumanPlayer(side);
    newGame(depth, side);
  }

  const status = game
    ? isViewingPast
      ? `Viewing move ${cursor + 1} of ${history.length}`
      : game.gameOver
        ? game.winner === "Draw"
          ? "Draw"
          : `${game.winner} wins`
        : busy
          ? "AI thinking"
          : game.currentPlayer === humanPlayer
            ? `${humanPlayer} to move`
            : "AI to move"
    : "Loading";

  return h(
    React.Fragment,
    null,
    h(
      "main",
      { className: `shell ${busy ? "is-thinking" : ""} ${newGamePulse ? "new-game-pulse" : ""}` },
      h("div", { className: "ambient ambient-one" }),
      h("div", { className: "ambient ambient-two" }),
      h("div", { className: "ambient ambient-three" }),
      h(
        "section",
        { className: "play-area" },
        h(
          "header",
          { className: "topbar" },
          h(
            "div",
            { className: "brand-block" },
            h("h1", null, h("span", null, "Reversi"), " Engine"),
            h("p", null, status)
          ),
          h(
            "div",
            { className: "controls" },
            h(SidePicker, { humanPlayer, busy, onPick: chooseSide }),
            h(DepthPicker, {
              depth,
              busy,
              open: depthOpen,
              setOpen: setDepthOpen,
              onPick: (value) => setDepth(value),
            }),
            h(
              "button",
              { type: "button", className: "new-game-button", disabled: busy, onClick: () => newGame(depth, humanPlayer) },
              h("span", null, "New game")
            )
          )
        ),
        h(BoardView, { game, legalSet, canPlay, onMove: playMove }),
        h(HistoryNav, { canGoBack, canGoForward, cursor, total: history.length, setCursor }),
        busy && h(ThinkingDock, null)
      ),
      h(Sidebar, { game, liveGame, busy, error, isViewingPast, humanPlayer })
    ),
    h(SiteFooter, null)
  );
}

function SidePicker({ humanPlayer, busy, onPick }) {
  return h(
    "div",
    { className: "side-picker", "aria-label": "Choose side" },
    SIDES.map((side) =>
      h(
        "button",
        {
          key: side,
          type: "button",
          className: side === humanPlayer ? "active" : "",
          disabled: busy,
          onClick: () => onPick(side),
        },
        h("span", { className: `mini-disc ${side.toLowerCase()}` }),
        side
      )
    )
  );
}

function DepthPicker({ depth, busy, open, setOpen, onPick }) {
  return h(
    "div",
    { className: "depth-picker" },
    h("span", { className: "control-label" }, "Depth"),
    h(
      "button",
      {
        type: "button",
        className: `depth-trigger ${open ? "open" : ""}`,
        disabled: busy,
        onClick: () => setOpen(!open),
      },
      h("strong", null, depth),
      h("span", { className: "chevron" })
    ),
    open &&
      h(
        "div",
        { className: "depth-menu" },
        DEPTHS.map((value) =>
          h(
            "button",
            {
              key: value,
              type: "button",
              className: value === depth ? "selected" : "",
              onClick: () => {
                onPick(value);
                setOpen(false);
              },
            },
            h("strong", null, value),
            h("small", null, value < 7 ? "fast" : value === 7 ? "balanced" : "deep")
          )
        )
      )
  );
}

function BoardView({ game, legalSet, canPlay, onMove }) {
  const cells = game?.cells ?? Array(64).fill("empty");

  return h(
    "div",
    { className: "board-wrap" },
    h(
      "div",
      { className: "files files-top" },
      FILES.map((file) => h("span", { key: file }, file))
    ),
    h(
      "div",
      { className: "board-row-layout" },
      h("div", { className: "ranks" }, Array.from({ length: 8 }, (_, index) => h("span", { key: index }, index + 1))),
      h(
        "div",
        { className: "board", "aria-label": "Reversi board" },
        cells.map((cell, index) => {
          const coord = coordFor(index);
          const legal = canPlay && legalSet.has(coord);
          const className = ["square", legal ? "legal" : "", game?.lastHumanMove === coord ? "last" : ""]
            .filter(Boolean)
            .join(" ");

          return h(
            "button",
            {
              key: coord,
              type: "button",
              className,
              disabled: !legal || game?.gameOver,
              onClick: () => onMove(coord),
              title: legal ? `Play ${coord}` : coord,
            },
            cell !== "empty" && h("span", { className: `disc ${cell}` }),
            legal && h("span", { className: "legal-orbit" })
          );
        })
      ),
      h("div", { className: "ranks" }, Array.from({ length: 8 }, (_, index) => h("span", { key: index }, index + 1)))
    ),
    h(
      "div",
      { className: "files files-bottom" },
      FILES.map((file) => h("span", { key: file }, file))
    )
  );
}

function HistoryNav({ canGoBack, canGoForward, cursor, total, setCursor }) {
  return h(
    "div",
    { className: "history-nav" },
    h(
      "button",
      {
        type: "button",
        className: "arrow-button left",
        disabled: !canGoBack,
        onClick: () => setCursor((value) => Math.max(0, value - 1)),
        title: "Previous position",
      },
      h("span", null)
    ),
    h("div", { className: "timeline" }, h("span", { style: { width: total > 1 ? `${(cursor / (total - 1)) * 100}%` : "0%" } })),
    h(
      "button",
      {
        type: "button",
        className: "arrow-button right",
        disabled: !canGoForward,
        onClick: () => setCursor((value) => Math.min(total - 1, value + 1)),
        title: "Next position",
      },
      h("span", null)
    )
  );
}

function ThinkingDock() {
  return h(
    "div",
    { className: "thinking-dock", role: "status", "aria-live": "polite" },
    h("div", { className: "search-core" }, h("span", null), h("span", null), h("span", null)),
    h("div", { className: "thinking-copy" }, h("strong", null, "Searching lines"), h("small", null, "Alpha-beta pruning")),
    h("div", { className: "search-bars" }, Array.from({ length: 9 }, (_, index) => h("span", { key: index })))
  );
}

function Sidebar({ game, liveGame, busy, error, isViewingPast, humanPlayer }) {
  const source = game ?? liveGame;
  const black = source?.score?.black ?? 2;
  const white = source?.score?.white ?? 2;
  const legalMoves = source?.legalMoves ?? [];
  const messages = source?.messages ?? [];

  return h(
    "aside",
    { className: "sidebar" },
    h("div", { className: "sidebar-glow" }),
    h("h2", null, "Game"),
    h("p", { className: "side-note" }, `You are ${humanPlayer}${isViewingPast ? " - review mode" : ""}`),
    h(
      "div",
      { className: "score" },
      h("div", null, h("span", { className: "score-disc black" }), h("strong", null, black), h("small", null, "Black")),
      h("div", null, h("span", { className: "score-disc white" }), h("strong", null, white), h("small", null, "White"))
    ),
    h("h3", null, "Legal moves"),
    h(
      "div",
      { className: "moves" },
      legalMoves.length
        ? legalMoves.map((move) => h("span", { key: move }, move))
        : h("span", null, source?.gameOver ? "none" : "pass")
    ),
    error && h("p", { className: "error" }, error),
    busy &&
      h(
        "div",
        { className: "engine-loader" },
        h("div", { className: "loader-grid" }, Array.from({ length: 16 }, (_, index) => h("span", { key: index }))),
        h("p", { className: "thinking" }, "Engine is exploring")
      ),
    h("h3", null, "Log"),
    h(
      "ol",
      { className: "log" },
      messages.length
        ? messages.map((message, index) => h("li", { key: `${index}-${message}` }, message))
        : h("li", null, "Select a highlighted square when it is your turn.")
    )
  );
}

function SiteFooter() {
  return h(
    "footer",
    { className: "site-footer" },
    h(
      "div",
      { className: "footer-inner" },
      h(
        "a",
        {
          href: "https://github.com/itskushagraa/reversi-engine",
          target: "_blank",
          rel: "noreferrer",
          className: "footer-link github-link",
        },
        h("i", { className: "footer-icon", "data-lucide": "github", "aria-hidden": "true" }),
        h("span", null, "read the source")
      ),
      h(
        "span",
        { className: "footer-copy" },
        h("i", { className: "footer-icon", "data-lucide": "copyright", "aria-hidden": "true" }),
        "2026 ",
        h("strong", null, "Kushagra Sharma")
      ),
      h(
        "a",
        {
          href: "https://kush-sharma.com",
          target: "_blank",
          rel: "noreferrer",
          className: "footer-link site-link",
        },
        h("i", { className: "footer-icon", "data-lucide": "sparkles", "aria-hidden": "true" }),
        h("span", null, "see what else i build")
      )
    ),
  );
}

function renderLucideIcons() {
  if (window.lucide) {
    window.lucide.createIcons({
      attrs: {
        "stroke-width": 2.2,
      },
    });
  }
}

ReactDOM.createRoot(document.getElementById("root")).render(h(App));
