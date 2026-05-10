const { createElement: h, useEffect, useMemo, useState } = React;

const FILES = ["a", "b", "c", "d", "e", "f", "g", "h"];
const DEPTHS = [3, 5, 7, 9, 11];
const SIDES = ["Black", "White"];
const TURN_FRAME_DELAY_MS = 650;

// Standard Reversi starting position cell array (index = bit index in bitboard)
const STANDARD_CELLS = (() => {
  const c = Array(64).fill("empty");
  c[27] = "white"; c[28] = "black"; c[35] = "black"; c[36] = "white";
  return c;
})();

// Convert a 64-element cell array to {black, white} bitboard strings.
// Uses BigInt to avoid JS precision loss on 64-bit integers.
function cellsToBitboards(cells) {
  let black = 0n, white = 0n;
  for (let i = 0; i < 64; i++) {
    if (cells[i] === "black") black |= 1n << BigInt(i);
    else if (cells[i] === "white") white |= 1n << BigInt(i);
  }
  return { black: black.toString(), white: white.toString() };
}

function coordFor(index) {
  return `${FILES[index % 8]}${Math.floor(index / 8) + 1}`;
}

function wait(ms) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function App() {
  // ── Shared state ────────────────────────────────────────────────────────────
  const [appMode, setAppMode] = useState("play"); // "play" | "analyze"
  const [depth, setDepth] = useState(7);
  const [humanPlayer, setHumanPlayer] = useState("Black");
  const [depthOpen, setDepthOpen] = useState(false);

  // ── Play mode state ─────────────────────────────────────────────────────────
  const [history, setHistory] = useState([]);
  const [cursor, setCursor] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [newGamePulse, setNewGamePulse] = useState(false);

  // ── Analyze mode state ──────────────────────────────────────────────────────
  const [editorCells, setEditorCells] = useState(() => [...STANDARD_CELLS]);
  const [editorTool, setEditorTool] = useState("black");
  const [editorPlayer, setEditorPlayer] = useState("Black");
  const [analysisResult, setAnalysisResult] = useState(null);
  const [analysisBusy, setAnalysisBusy] = useState(false);
  const [analyzeError, setAnalyzeError] = useState("");

  const game = history[cursor] ?? null;
  const liveGame = history[history.length - 1] ?? null;
  const isViewingPast = history.length > 0 && cursor < history.length - 1;
  const canGoBack = cursor > 0;
  const canGoForward = cursor < history.length - 1;

  useEffect(() => {
    newGame(depth, humanPlayer);
  }, []);

  useEffect(() => {
    if (window.lucide) {
      window.lucide.createIcons();
    }
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
      await showResponseFrames(data, { replace: true });
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
      await showResponseFrames(data, { baseCursor: cursor });
    } catch (err) {
      setError(err.message);
    } finally {
      setBusy(false);
    }
  }

  async function showResponseFrames(data, { replace = false, baseCursor = cursor } = {}) {
    const frames = Array.isArray(data.frames) && data.frames.length ? data.frames : [data];
    const shown = [];

    for (const frame of frames) {
      if (shown.length > 0) {
        await wait(TURN_FRAME_DELAY_MS);
      }

      shown.push(frame);
      setHistory((items) => {
        const base = replace ? [] : items.slice(0, baseCursor + 1);
        const next = base.concat(shown);
        setCursor(next.length - 1);
        return next;
      });
    }
  }

  function chooseSide(side) {
    setHumanPlayer(side);
    newGame(depth, side);
  }

  // ── Analyze mode handlers ─────────────────────────────────────────────────

  function handleEditorCellClick(index) {
    setEditorCells((prev) => {
      const next = [...prev];
      next[index] = editorTool === "erase" ? "empty" : editorTool;
      return next;
    });
    setAnalysisResult(null);
  }

  function changeEditorPlayer(side) {
    setEditorPlayer(side);
    setAnalysisResult(null);
  }

  function clearBoard() {
    setEditorCells(Array(64).fill("empty"));
    setAnalysisResult(null);
    setAnalyzeError("");
  }

  function resetToStart() {
    setEditorCells([...STANDARD_CELLS]);
    setAnalysisResult(null);
    setAnalyzeError("");
    setEditorPlayer("Black");
  }

  async function runAnalysis() {
    setAnalysisBusy(true);
    setAnalyzeError("");
    const { black, white } = cellsToBitboards(editorCells);
    try {
      const res = await fetch("/api/analyze", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ black, white, currentPlayer: editorPlayer, depth }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Analysis failed");
      setAnalysisResult(data);
    } catch (err) {
      setAnalyzeError(err.message);
    } finally {
      setAnalysisBusy(false);
    }
  }

  async function playFromHere() {
    setAnalysisBusy(true);
    setAnalyzeError("");
    const { black, white } = cellsToBitboards(editorCells);
    try {
      const res = await fetch("/api/start", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ black, white, currentPlayer: editorPlayer, humanPlayer, depth }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Could not start game");
      setAppMode("play");
      await showResponseFrames(data, { replace: true });
    } catch (err) {
      setAnalyzeError(err.message);
    } finally {
      setAnalysisBusy(false);
    }
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

  const isThinking = busy || analysisBusy;

  return h(
    React.Fragment,
    null,
    h(
      "main",
      { className: `shell ${appMode === "analyze" ? "is-analyze-mode" : "is-play-mode"} ${isThinking ? "is-thinking" : ""} ${newGamePulse ? "new-game-pulse" : ""}` },
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
            h("p", null, appMode === "play" ? status : "Position editor")
          ),
          appMode === "play" &&
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
        appMode === "play"
          ? h(
              React.Fragment,
              null,
              h(BoardView, { game, legalSet, canPlay, onMove: playMove }),
              h(HistoryNav, { canGoBack, canGoForward, cursor, total: history.length, setCursor }),
              busy && h(ThinkingDock, null)
            )
          : h(AnalyzeBoardView, {
              cells: editorCells,
              editorTool,
              analysisResult,
              evalScore: analysisResult?.eval ?? 0,
              onCellClick: handleEditorCellClick,
            })
      ),
      appMode === "play"
        ? h(Sidebar, { game, liveGame, busy, error, isViewingPast, humanPlayer, appMode, onModeChange: setAppMode })
        : h(AnalyzeSidebar, {
            analysisResult,
            editorCells,
            analysisBusy,
            analyzeError,
            editorTool,
            onTool: (t) => setEditorTool(t),
            editorPlayer,
            onEditorPlayer: changeEditorPlayer,
            humanPlayer,
            onHumanPlayer: setHumanPlayer,
            depth,
            onDepth: setDepth,
            depthOpen,
            setDepthOpen,
            onRunAnalysis: runAnalysis,
            onPlayFromHere: playFromHere,
            onClear: clearBoard,
            onReset: resetToStart,
            appMode,
            onModeChange: setAppMode,
          })
    ),
    h(BottomLinks, null)
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

function EvalBar({ evalScore }) {
  const clamped = Math.max(-10, Math.min(10, evalScore ?? 0));
  // Chess convention: positive = White winning, negative = Black winning.
  // Black fills top of bar — more black fill = Black is winning (clamped is more negative).
  const blackPct = ((10 - clamped) / 20) * 100;
  const display =
    Math.abs(clamped) < 0.05
      ? "0.0"
      : clamped > 0
        ? `+${clamped.toFixed(1)}`
        : clamped.toFixed(1);
  const evalState = clamped < -1.5 ? "black" : clamped > 1.5 ? "white" : "equal";

  return h(
    "div",
    {
      className: "eval-bar-outer",
      style: { "--eval-black-pct": `${blackPct.toFixed(2)}%` },
      "data-eval": evalState,
      role: "meter",
      "aria-label": `Position evaluation: ${display}`,
      "aria-valuenow": clamped.toFixed(1),
      "aria-valuemin": "-10",
      "aria-valuemax": "10",
    },
    h(
      "div",
      { className: "eval-bar-track" },
      h("div", { className: "eval-bar-black" }),
      h("div", { className: "eval-bar-white" })
    ),
    h("div", { className: "eval-bar-separator" }),
    h("div", { className: "eval-bar-label" }, display)
  );
}

function BoardView({ game, legalSet, canPlay, onMove }) {
  const cells = game?.cells ?? Array(64).fill("empty");
  const evalScore = game?.eval ?? 0;

  return h(
    "div",
    { className: "board-outer" },
    h(EvalBar, { evalScore }),
    h(
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

function Sidebar({ game, liveGame, busy, error, isViewingPast, humanPlayer, appMode, onModeChange }) {
  const source = game ?? liveGame;
  const black = source?.score?.black ?? 2;
  const white = source?.score?.white ?? 2;
  const legalMoves = source?.legalMoves ?? [];
  const messages = source?.messages ?? [];

  return h(
    "aside",
    { className: "sidebar" },
    h("div", { className: "sidebar-glow" }),
    h(ModeToggle, { mode: appMode, onChange: onModeChange }),
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

// ── Analyze mode components ──────────────────────────────────────────────────

function ModeToggle({ mode, onChange }) {
  return h(
    "div",
    { className: "mode-toggle", "aria-label": "Select mode" },
    ["play", "analyze"].map((m) =>
      h(
        "button",
        {
          key: m,
          type: "button",
          className: m === mode ? "active" : "",
          onClick: () => onChange(m),
        },
        m === "play" ? "Play" : "Analyze"
      )
    )
  );
}

function EditorToolbar({ editorTool, onTool, editorPlayer, onEditorPlayer }) {
  return h(
    "div",
    { className: "editor-toolbar" },
    h(
      "div",
      { className: "editor-tool-section" },
      h("span", { className: "control-label" }, "Place"),
      h(
        "div",
        { className: "editor-tool-group" },
        [
          { id: "black", label: "Black" },
          { id: "white", label: "White" },
          { id: "erase", label: "Erase" },
        ].map(({ id, label }) =>
          h(
            "button",
            {
              key: id,
              type: "button",
              className: `editor-tool-btn ${editorTool === id ? "active" : ""}`,
              onClick: () => onTool(id),
              title: label,
            },
            id !== "erase"
              ? h("span", { className: `mini-disc ${id}` })
              : h("span", { className: "erase-x", "aria-hidden": "true" }),
            h("span", null, label)
          )
        )
      )
    ),
    h(
      "div",
      { className: "editor-tool-section" },
      h("span", { className: "control-label" }, "To play"),
      h(
        "div",
        { className: "editor-tool-group turn-group" },
        SIDES.map((side) =>
          h(
            "button",
            {
              key: side,
              type: "button",
              className: `editor-tool-btn ${editorPlayer === side ? "active" : ""}`,
              onClick: () => onEditorPlayer(side),
            },
            h("span", { className: `mini-disc ${side.toLowerCase()}` }),
            side
          )
        )
      )
    )
  );
}

function AnalyzeBoardView({ cells, editorTool, analysisResult, evalScore, onCellClick }) {
  const bestMove = analysisResult?.aiMove ?? null;
  const legalSet = new Set(analysisResult?.legalMoves ?? []);

  return h(
    "div",
    { className: "board-outer" },
    h(EvalBar, { evalScore }),
    h(
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
        h(
          "div",
          { className: "ranks" },
          Array.from({ length: 8 }, (_, i) => h("span", { key: i }, i + 1))
        ),
        h(
          "div",
          { className: "board editor-mode", "aria-label": "Position editor" },
          cells.map((cell, index) => {
            const coord = coordFor(index);
            const isBest = coord === bestMove;
            const isLegal = legalSet.has(coord) && !isBest;
            const isPlaceable = editorTool !== "erase" && cell === "empty" && !isBest;
            const className = [
              "square",
              "editor-sq",
              isBest ? "editor-best-move" : "",
              isLegal ? "editor-legal" : "",
              isPlaceable ? "editor-placeable" : "",
            ]
              .filter(Boolean)
              .join(" ");

            return h(
              "button",
              {
                key: coord,
                type: "button",
                className,
                onClick: () => onCellClick(index),
                title: `${coord}${isBest ? " — AI best move" : ""}`,
              },
              cell !== "empty" && h("span", { className: `disc ${cell}` }),
              isBest && h("span", { className: "best-move-ring" }),
              isLegal && h("span", { className: "legal-orbit editor-legal-orbit" })
            );
          })
        ),
        h(
          "div",
          { className: "ranks" },
          Array.from({ length: 8 }, (_, i) => h("span", { key: i }, i + 1))
        )
      ),
      h(
        "div",
        { className: "files files-bottom" },
        FILES.map((file) => h("span", { key: file }, file))
      )
    )
  );
}

function AnalysisActions({ analysisBusy, analyzeError, onRunAnalysis, onPlayFromHere, onClear, onReset }) {
  return h(
    "div",
    { className: "analysis-actions" },
    h(
      "div",
      { className: "analysis-primary" },
      h(
        "button",
        {
          type: "button",
          className: "run-analysis-btn",
          disabled: analysisBusy,
          onClick: onRunAnalysis,
        },
        analysisBusy
          ? h(React.Fragment, null, h("span", { className: "btn-spinner" }), "Analyzing…")
          : "Run Analysis"
      ),
      h(
        "button",
        {
          type: "button",
          className: "play-from-here-btn",
          disabled: analysisBusy,
          onClick: onPlayFromHere,
        },
        "▶ Play from here"
      )
    ),
    h(
      "div",
      { className: "analysis-secondary" },
      h(
        "button",
        { type: "button", className: "editor-util-btn", disabled: analysisBusy, onClick: onClear },
        "Clear board"
      ),
      h(
        "button",
        { type: "button", className: "editor-util-btn", disabled: analysisBusy, onClick: onReset },
        "Reset to start"
      )
    ),
    analyzeError && h("p", { className: "error" }, analyzeError)
  );
}

function AnalyzeSidebar({
  analysisResult, editorCells, analysisBusy, analyzeError,
  editorTool, onTool, editorPlayer, onEditorPlayer,
  humanPlayer, onHumanPlayer, depth, onDepth, depthOpen, setDepthOpen,
  onRunAnalysis, onPlayFromHere, onClear, onReset,
  appMode, onModeChange,
}) {
  const discCount = (color) => editorCells.filter((c) => c === color).length;
  const black = analysisResult?.score?.black ?? discCount("black");
  const white = analysisResult?.score?.white ?? discCount("white");
  const hasResult = !!analysisResult;

  const evalScore = analysisResult?.eval;
  const evalLabel =
    evalScore === undefined ? null
    : Math.abs(evalScore) < 0.05 ? "Equal"
    : evalScore < 0 ? `Black +${Math.abs(evalScore).toFixed(1)}`
    : `White +${evalScore.toFixed(1)}`;

  const legalMoves = analysisResult?.legalMoves ?? [];
  const bestMove = analysisResult?.aiMove ?? null;

  return h(
    "aside",
    { className: "sidebar sidebar--analyze" },
    h("div", { className: "sidebar-glow" }),
    h(ModeToggle, { mode: appMode, onChange: onModeChange }),
    h("h2", null, "Analysis"),

    // Disc counts
    h(
      "div",
      { className: "score" },
      h("div", null, h("span", { className: "score-disc black" }), h("strong", null, black), h("small", null, "Black")),
      h("div", null, h("span", { className: "score-disc white" }), h("strong", null, white), h("small", null, "White"))
    ),

    // Place tool
    h(
      "div",
      { className: "analyze-control-group" },
      h("span", { className: "control-label" }, "Place disc"),
      h(
        "div",
        { className: "editor-tool-group" },
        [
          { id: "black", label: "Black" },
          { id: "white", label: "White" },
          { id: "erase", label: "Erase" },
        ].map(({ id, label }) =>
          h(
            "button",
            {
              key: id,
              type: "button",
              className: `editor-tool-btn ${editorTool === id ? "active" : ""}`,
              disabled: analysisBusy,
              onClick: () => onTool(id),
            },
            id !== "erase" ? h("span", { className: `mini-disc ${id}` }) : h("span", { className: "erase-x" }),
            h("span", null, label)
          )
        )
      )
    ),

    // Turn + play-as row
    h(
      "div",
      { className: "analyze-row" },
      h(
        "div",
        { className: "analyze-control-group" },
        h("span", { className: "control-label" }, "To play"),
        h(
          "div",
          { className: "editor-tool-group turn-group" },
          SIDES.map((side) =>
            h(
              "button",
              {
                key: side,
                type: "button",
                className: `editor-tool-btn ${editorPlayer === side ? "active" : ""}`,
                disabled: analysisBusy,
                onClick: () => onEditorPlayer(side),
              },
              h("span", { className: `mini-disc ${side.toLowerCase()}` }),
              side
            )
          )
        )
      ),
      h(
        "div",
        { className: "analyze-control-group" },
        h("span", { className: "control-label" }, "You play"),
        h(
          "div",
          { className: "editor-tool-group turn-group" },
          SIDES.map((side) =>
            h(
              "button",
              {
                key: side,
                type: "button",
                className: `editor-tool-btn ${humanPlayer === side ? "active" : ""}`,
                disabled: analysisBusy,
                onClick: () => onHumanPlayer(side),
              },
              h("span", { className: `mini-disc ${side.toLowerCase()}` }),
              side
            )
          )
        )
      )
    ),

    // Depth
    h(DepthPicker, { depth, busy: analysisBusy, open: depthOpen, setOpen: setDepthOpen, onPick: onDepth }),

    // Primary actions
    h(
      "div",
      { className: "analysis-primary" },
      h(
        "button",
        { type: "button", className: "run-analysis-btn", disabled: analysisBusy, onClick: onRunAnalysis },
        analysisBusy
          ? h(React.Fragment, null, h("span", { className: "btn-spinner" }), "Analyzing…")
          : "Run Analysis"
      ),
      h(
        "button",
        { type: "button", className: "play-from-here-btn", disabled: analysisBusy, onClick: onPlayFromHere },
        "▶ Play from here"
      )
    ),

    // Utility actions
    h(
      "div",
      { className: "analysis-secondary" },
      h("button", { type: "button", className: "editor-util-btn", disabled: analysisBusy, onClick: onClear }, "Clear board"),
      h("button", { type: "button", className: "editor-util-btn", disabled: analysisBusy, onClick: onReset }, "Reset to start")
    ),

    analyzeError && h("p", { className: "error" }, analyzeError),

    // Analysis results
    hasResult &&
      h(
        React.Fragment,
        null,
        h("div", { className: "analyze-divider" }),
        h(
          "div",
          { className: "analyze-result-row" },
          h("span", { className: "control-label" }, "Evaluation"),
          h("span", { className: "analysis-eval-text" }, evalLabel)
        ),
        h(
          "div",
          { className: "analyze-result-row" },
          h("span", { className: "control-label" }, "AI suggests"),
          h(
            "div",
            { className: "analysis-best-move" },
            analysisResult.gameOver
              ? h("span", { className: "analysis-hint" }, "Game over")
              : bestMove
                ? h("span", { className: "best-move-badge" }, bestMove)
                : h("span", { className: "analysis-hint" }, "Pass")
          )
        ),
        h("h3", null, "Legal moves"),
        h(
          "div",
          { className: "moves" },
          legalMoves.length
            ? legalMoves.map((m) =>
                h("span", { key: m, className: m === bestMove ? "move-best" : "" }, m)
              )
            : h("span", null, analysisResult.gameOver ? "none" : "pass")
        )
      ),

    analysisBusy &&
      h(
        "div",
        { className: "engine-loader" },
        h("div", { className: "loader-grid" }, Array.from({ length: 16 }, (_, i) => h("span", { key: i }))),
        h("p", { className: "thinking" }, "Engine is searching")
      ),

    !hasResult && !analysisBusy &&
      h("p", { className: "analyze-tip-inline" }, "Run analysis to see the engine evaluation and best move.")
  );
}

// ── End analyze mode components ───────────────────────────────────────────────

function BottomLinks() {
  return h(
    "div",
    { className: "bottom-links", "aria-label": "project links" },
    h(
      "a",
      {
        href: "https://github.com/itskushagraa/reversi-engine",
        target: "_blank",
        rel: "noreferrer",
        className: "bottom-link source-link",
      },
      h(LucideGithub, null),
      h("span", null, "read the source")
    ),
    h(
      "span",
      { className: "bottom-link bottom-copy" },
      h(LucideCopyright, null),
      h("span", null, "2026 "),
      h("strong", null, "Kushagra Sharma")
    ),
    h(
      "a",
      {
        href: "https://kush-sharma.com",
        target: "_blank",
        rel: "noreferrer",
        className: "bottom-link site-link",
      },
      h("i", { className: "footer-icon", "data-lucide": "sparkles", "aria-hidden": "true" }),
      h("span", null, "see what else i build")
    )
  );
}

function LucideGithub() {
  return h(
    "svg",
    {
      className: "footer-icon lucide lucide-github",
      xmlns: "http://www.w3.org/2000/svg",
      width: "24",
      height: "24",
      viewBox: "0 0 24 24",
      fill: "none",
      stroke: "currentColor",
      strokeWidth: "2.2",
      strokeLinecap: "round",
      strokeLinejoin: "round",
      "aria-hidden": "true",
    },
    h("path", {
      d: "M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4",
    }),
    h("path", { d: "M9 18c-4.51 2-5-2-7-2" })
  );
}

function LucideCopyright() {
  return h(
    "svg",
    {
      className: "footer-icon lucide lucide-copyright",
      xmlns: "http://www.w3.org/2000/svg",
      width: "24",
      height: "24",
      viewBox: "0 0 24 24",
      fill: "none",
      stroke: "currentColor",
      strokeWidth: "2.2",
      strokeLinecap: "round",
      strokeLinejoin: "round",
      "aria-hidden": "true",
    },
    h("circle", { cx: "12", cy: "12", r: "10" }),
    h("path", { d: "M14.83 9.17a4 4 0 1 0 0 5.66" })
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(h(App));
