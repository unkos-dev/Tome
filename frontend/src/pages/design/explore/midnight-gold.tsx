import { useState, type ReactElement, type CSSProperties } from "react";
import { Link } from "react-router";
import "../../../design/explore/midnight-gold/tokens.css";
import {
  BOOKS,
  SHELVES,
  STATS,
  USER_SHELVES,
  bookHue,
  bookTier,
  type Book,
} from "./_shared/books";

type Theme = "dark" | "light";
type Mock = "home" | "detail" | "library";
type GridSize = "s" | "m" | "l";
type ViewMode = "grid" | "table";

const COVER_PALETTES_DARK = [
  ["#3B2A18", "#1F1812"],
  ["#2C2418", "#15110B"],
  ["#3A2D20", "#1A1410"],
  ["#28201A", "#161210"],
  ["#33291B", "#1C160F"],
];

const COVER_PALETTES_LIGHT = [
  ["#D8C394", "#A98A4E"],
  ["#E2D2A9", "#B89A60"],
  ["#CDB888", "#9A7E45"],
  ["#E8DBB6", "#C0A668"],
  ["#D5BE92", "#A88955"],
];

function coverStyle(book: Book, theme: Theme): CSSProperties {
  const hue = bookHue(book.id);
  const tier = bookTier(book.id);
  const palette = theme === "dark" ? COVER_PALETTES_DARK : COVER_PALETTES_LIGHT;
  const [a, b] = palette[tier];
  const angle = (hue % 90) + 135;
  return {
    background: `linear-gradient(${angle}deg, ${a} 0%, ${b} 80%)`,
    color: theme === "dark" ? "#ECE3D0" : "#2A231A",
  };
}

function coverBackdropStyle(book: Book): CSSProperties {
  const hue = bookHue(book.id);
  return {
    background: `radial-gradient(60% 50% at 30% 30%, hsl(${hue}, 35%, 18%), transparent 70%),
                 radial-gradient(50% 50% at 75% 60%, hsl(${(hue + 60) % 360}, 30%, 22%), transparent 70%),
                 #14130E`,
  };
}

interface CoverProps {
  book: Book;
  theme: Theme;
  showAuthor?: boolean;
}

function Cover({ book, theme, showAuthor = true }: CoverProps): ReactElement {
  return (
    <div className="mg-tile-cover">
      <div className="mg-tile-cover-inner" style={coverStyle(book, theme)}>
        <div>
          <div className="mg-tile-cover-rule" />
          {showAuthor && <div className="mg-tile-cover-author">{book.author}</div>}
        </div>
        <div className="mg-tile-cover-title">{book.title}</div>
      </div>
    </div>
  );
}

interface TileProps {
  book: Book;
  theme: Theme;
  size?: "default" | "lg" | "sm";
  showStatus?: boolean;
}

function Tile({ book, theme, size = "default", showStatus = false }: TileProps): ReactElement {
  const cls = size === "lg" ? "mg-tile mg-tile-lg" : size === "sm" ? "mg-tile mg-tile-sm" : "mg-tile";
  return (
    <article className={cls}>
      <Cover book={book} theme={theme} />
      <div className="mg-tile-meta">
        <h4 className="mg-tile-title">{book.title}</h4>
        <div className="mg-tile-author">{book.author}</div>
        {book.status === "in-progress" && book.progress !== undefined && (
          <div className="mg-tile-progress" aria-label={`${Math.round(book.progress * 100)}% read`}>
            <div className="mg-tile-progress-fill" style={{ width: `${book.progress * 100}%` }} />
          </div>
        )}
        {showStatus && book.status !== "unread" && (
          <div className="mg-tile-status">
            <span className={`mg-status-dot ${book.status}`} />
            {book.status === "in-progress"
              ? `${Math.round((book.progress ?? 0) * 100)}% read`
              : "Finished"}
          </div>
        )}
      </div>
    </article>
  );
}

interface HomeProps {
  theme: Theme;
}

function Home({ theme }: HomeProps): ReactElement {
  const featured = SHELVES.inProgress[0] ?? BOOKS[0];
  return (
    <>
      <section className="mg-hero">
        <div className="mg-hero-backdrop" style={coverBackdropStyle(featured)} />
        <div className="mg-hero-content">
          <div className="mg-eyebrow">Wednesday evening · Continue</div>
          <h1 className="mg-hero-h">
            Two pages back into <em>{featured.title}</em>.
          </h1>
          <p className="mg-hero-sub">
            You left off at chapter twelve. {Math.round((featured.progress ?? 0) * 100)} percent of
            the way through. Pick it up where you stopped.
          </p>
          <button className="mg-hero-cta" type="button">
            Resume reading
            <span aria-hidden="true">→</span>
          </button>
          <div className="mg-hero-meta">
            <span><strong>{STATS.totalBooks.toLocaleString()}</strong> books in library</span>
            <span><strong>{STATS.inProgress}</strong> in progress</span>
            <span><strong>{STATS.finishedThisYear}</strong> finished this year</span>
          </div>
        </div>
      </section>

      <section className="mg-shelf">
        <div className="mg-shelf-head">
          <h2 className="mg-shelf-title">In progress</h2>
          <div className="mg-shelf-meta">
            <span>{SHELVES.inProgress.length} active</span>
          </div>
        </div>
        <div className="mg-carousel">
          {SHELVES.inProgress.map((b) => (
            <Tile key={b.id} book={b} theme={theme} size="lg" showStatus />
          ))}
        </div>
      </section>

      <section className="mg-shelf">
        <div className="mg-shelf-head">
          <h2 className="mg-shelf-title">Recently added</h2>
          <div className="mg-shelf-meta">
            <a href="#">See all →</a>
          </div>
        </div>
        <div className="mg-carousel">
          {SHELVES.recentlyAdded.map((b) => (
            <Tile key={b.id} book={b} theme={theme} />
          ))}
        </div>
      </section>

      <section className="mg-shelf">
        <div className="mg-shelf-head">
          <h2 className="mg-shelf-title">
            Forgotten favourites <em>·</em> from your library
          </h2>
          <div className="mg-shelf-meta">
            <span className="mg-badge">Discovery</span>
          </div>
        </div>
        <div className="mg-carousel">
          {SHELVES.forgotten.map((b) => (
            <Tile key={b.id} book={b} theme={theme} />
          ))}
        </div>
      </section>

      <section className="mg-shelf">
        <div className="mg-shelf-head">
          <h2 className="mg-shelf-title">Lantern Cycle <em>·</em> incomplete series</h2>
          <div className="mg-shelf-meta">
            <span className="mg-badge">Smart shelf</span>
          </div>
        </div>
        <div className="mg-carousel">
          {SHELVES.byYusra.map((b) => (
            <Tile key={b.id} book={b} theme={theme} showStatus />
          ))}
        </div>
      </section>

      <section className="mg-stats">
        <div>
          <div className="mg-stat-num">
            <em>{STATS.totalBooks.toLocaleString()}</em>
          </div>
          <div className="mg-stat-label">Books in library</div>
        </div>
        <div>
          <div className="mg-stat-num">{STATS.read}</div>
          <div className="mg-stat-label">Read all-time</div>
        </div>
        <div>
          <div className="mg-stat-num">{STATS.hoursThisYear}h</div>
          <div className="mg-stat-label">Read this year</div>
        </div>
        <div>
          <div className="mg-stat-num">{STATS.pagesThisYear.toLocaleString()}</div>
          <div className="mg-stat-label">Pages this year</div>
        </div>
      </section>
    </>
  );
}

function Detail({ theme }: { theme: Theme }): ReactElement {
  const book = BOOKS.find((b) => b.id === "b02") ?? BOOKS[0];
  const moreByAuthor = BOOKS.filter((b) => b.author === book.author && b.id !== book.id).slice(0, 4);
  return (
    <article className="mg-detail">
      <div className="mg-detail-backdrop" style={coverBackdropStyle(book)} />
      <div className="mg-detail-grid">
        <aside>
          <div className="mg-detail-cover">
            <div className="mg-tile-cover-inner" style={coverStyle(book, theme)}>
              <div>
                <div className="mg-tile-cover-rule" />
                <div className="mg-tile-cover-author">{book.author}</div>
              </div>
              <div className="mg-tile-cover-title">{book.title}</div>
            </div>
          </div>
          <dl className="mg-detail-aside">
            <div>
              <dt>Format</dt>
              <dd>{book.format.toUpperCase()} · {book.pages} pages</dd>
            </div>
            <div>
              <dt>Published</dt>
              <dd>{book.year}</dd>
            </div>
            <div>
              <dt>Added</dt>
              <dd>{book.addedDays} days ago</dd>
            </div>
            <div>
              <dt>Status</dt>
              <dd>
                {book.status === "in-progress"
                  ? `Reading · ${Math.round((book.progress ?? 0) * 100)}% complete`
                  : book.status === "finished"
                    ? "Finished"
                    : "Unread"}
              </dd>
            </div>
          </dl>
        </aside>
        <div>
          <div className="mg-detail-eyebrow">Reverie · Detail view</div>
          <h1 className="mg-detail-title">
            <em>Salt</em> and Cipher
          </h1>
          <p className="mg-detail-byline">by {book.author}</p>
          <div className="mg-detail-actions">
            <button className="mg-hero-cta" type="button">
              Resume at 78%
              <span aria-hidden="true">→</span>
            </button>
            <button className="mg-button-secondary" type="button">Add to shelf</button>
            <button className="mg-button-secondary" type="button">Send to Kobo</button>
          </div>
          <div className="mg-detail-summary">
            <p>
              An archivist on a coastal research station begins decoding a sequence of letters that
              appear, then disappear, in the salt residue at the bottom of the brackish-water jars
              kept along the south wall. The sender is unknown. The cipher resists every method she
              owns. Over six months she trains herself to wait — to let the letters resolve in the
              order they want — and the book turns, gradually, into a study of patient attention.
            </p>
            <p>
              A second narrative thread tracks her grandfather, who built the station in the 1960s
              and may have been the cipher's original author. The book moves between the two voices
              in measured intervals, and refuses to confirm.
            </p>
          </div>
          <section className="mg-detail-section">
            <h3>More by {book.author}</h3>
            <div className="mg-detail-row">
              {moreByAuthor.map((b) => (
                <Tile key={b.id} book={b} theme={theme} size="sm" />
              ))}
              {moreByAuthor.length === 0 && (
                <p style={{ color: "var(--mg-fg-faint)", fontSize: "var(--mg-type-small)" }}>
                  This is the only {book.author} title in your library.
                </p>
              )}
            </div>
          </section>
        </div>
      </div>
    </article>
  );
}

function Library({ theme }: { theme: Theme }): ReactElement {
  const [size, setSize] = useState<GridSize>("m");
  const [view, setView] = useState<ViewMode>("grid");
  const [shelf, setShelf] = useState<string>("All");

  const shelves = ["All", ...USER_SHELVES.map((s) => s.name)];

  return (
    <div className="mg-library">
      <div className="mg-library-head">
        <div>
          <div className="mg-eyebrow">{BOOKS.length} of {STATS.totalBooks.toLocaleString()}</div>
          <h1 className="mg-library-title">
            Your <em>library</em>
          </h1>
        </div>
        <div className="mg-library-controls">
          <div className="mg-control-group" role="group" aria-label="Tile size">
            <button type="button" aria-pressed={size === "s"} onClick={() => setSize("s")}>S</button>
            <button type="button" aria-pressed={size === "m"} onClick={() => setSize("m")}>M</button>
            <button type="button" aria-pressed={size === "l"} onClick={() => setSize("l")}>L</button>
          </div>
          <div className="mg-control-group" role="group" aria-label="View">
            <button type="button" aria-pressed={view === "grid"} onClick={() => setView("grid")}>Tiles</button>
            <button type="button" aria-pressed={view === "table"} onClick={() => setView("table")}>Table</button>
          </div>
          <button className="mg-button-secondary" type="button">+ New shelf</button>
        </div>
      </div>

      <div className="mg-library-shelves" role="tablist">
        {shelves.map((s) => {
          const meta = USER_SHELVES.find((u) => u.name === s);
          const icon = meta?.kind === "smart" ? "Auto" : meta?.kind === "device" ? "Sync" : null;
          return (
            <button
              key={s}
              type="button"
              role="tab"
              className="mg-shelf-chip"
              aria-pressed={shelf === s}
              onClick={() => setShelf(s)}
            >
              {icon && <span className="mg-shelf-chip-icon">{icon}</span>}
              {s}
              {meta && <span style={{ opacity: 0.5, marginLeft: 6 }}>{meta.count}</span>}
            </button>
          );
        })}
      </div>

      {view === "grid" ? (
        <div className="mg-grid" data-size={size}>
          {BOOKS.map((b) => (
            <Tile key={b.id} book={b} theme={theme} showStatus />
          ))}
        </div>
      ) : (
        <table className="mg-table">
          <thead>
            <tr>
              <th>Title</th>
              <th>Year</th>
              <th>Pages</th>
              <th>Status</th>
              <th>Added</th>
            </tr>
          </thead>
          <tbody>
            {BOOKS.map((b) => (
              <tr key={b.id}>
                <td>
                  <span className="mg-mini-cover" style={coverStyle(b, theme)} aria-hidden="true" />
                  <span style={{ verticalAlign: "middle" }}>
                    <span className="title">{b.title}</span>
                    <div className="author">{b.author}</div>
                  </span>
                </td>
                <td>{b.year}</td>
                <td>{b.pages}</td>
                <td>
                  <span className="mg-tile-status">
                    <span className={`mg-status-dot ${b.status}`} />
                    {b.status === "in-progress"
                      ? `${Math.round((b.progress ?? 0) * 100)}%`
                      : b.status === "finished" ? "Finished" : "Unread"}
                  </span>
                </td>
                <td>{b.addedDays}d</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

export default function MidnightGold(): ReactElement {
  const [theme, setTheme] = useState<Theme>("dark");
  const [mock, setMock] = useState<Mock>("home");

  return (
    <div className="mg-root" data-theme={theme}>
      <header className="mg-topbar">
        <div className="mg-wordmark">
          Reverie<span>.</span>
        </div>
        <nav className="mg-nav" aria-label="Primary">
          <a href="#" aria-current="page">Library</a>
          <a href="#">Shelves</a>
          <a href="#">Reader</a>
          <a href="#">Stats</a>
        </nav>
        <div className="mg-spacer" />
        <div className="mg-search" aria-label="Search">
          <span aria-hidden="true">⌕</span>
          <span>Search title, author, shelf</span>
          <kbd>⌘K</kbd>
        </div>
        <button className="mg-iconbtn" type="button" aria-label="Settings">⚙</button>
        <div className="mg-themetoggle" role="group" aria-label="Theme">
          <button type="button" aria-pressed={theme === "dark"} onClick={() => setTheme("dark")}>Dark</button>
          <button type="button" aria-pressed={theme === "light"} onClick={() => setTheme("light")}>Light</button>
        </div>
        <div className="mg-avatar" aria-label="User menu">JU</div>
      </header>

      <div className="mg-mocktabs" role="tablist" aria-label="Mock screen">
        <button type="button" role="tab" className="mg-mocktab" aria-pressed={mock === "home"} onClick={() => setMock("home")}>
          <span>01</span> Home dashboard
        </button>
        <button type="button" role="tab" className="mg-mocktab" aria-pressed={mock === "detail"} onClick={() => setMock("detail")}>
          <span>02</span> Book detail
        </button>
        <button type="button" role="tab" className="mg-mocktab" aria-pressed={mock === "library"} onClick={() => setMock("library")}>
          <span>03</span> Library full-grid
        </button>
        <div className="mg-spacer" />
        <Link
          to="/design/explore"
          style={{
            color: "var(--mg-fg-faint)",
            fontSize: "var(--mg-type-small)",
            letterSpacing: "var(--mg-tracking-chrome)",
            textTransform: "uppercase",
            textDecoration: "none",
            alignSelf: "center",
          }}
        >
          ← All directions
        </Link>
      </div>

      {mock === "home" && <Home theme={theme} />}
      {mock === "detail" && <Detail theme={theme} />}
      {mock === "library" && <Library theme={theme} />}
    </div>
  );
}
