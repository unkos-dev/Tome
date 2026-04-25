export type Book = {
  id: string;
  title: string;
  author: string;
  year: number;
  status: "unread" | "in-progress" | "finished";
  progress?: number;
  format: "epub" | "pdf";
  pages: number;
  addedDays: number;
  series?: { name: string; index: number; total: number };
};

export const BOOKS: Book[] = [
  { id: "b01", title: "The Glass Bell", author: "Anouk Verheven", year: 2021, status: "in-progress", progress: 0.42, format: "epub", pages: 312, addedDays: 4 },
  { id: "b02", title: "Salt and Cipher", author: "Idris Bekele", year: 2019, status: "in-progress", progress: 0.78, format: "epub", pages: 488, addedDays: 11 },
  { id: "b03", title: "The Kept Garden", author: "Mercer Hadley", year: 2023, status: "finished", format: "epub", pages: 264, addedDays: 19 },
  { id: "b04", title: "Lantern Pulse", author: "Yusra Al-Mansouri", year: 2022, status: "unread", format: "epub", pages: 401, addedDays: 2, series: { name: "Lantern Cycle", index: 1, total: 4 } },
  { id: "b05", title: "Quiet Continent", author: "Tomás Fialho", year: 2018, status: "finished", format: "epub", pages: 552, addedDays: 88 },
  { id: "b06", title: "Architecture of the Void", author: "Saskia Brandt", year: 2024, status: "unread", format: "pdf", pages: 196, addedDays: 1 },
  { id: "b07", title: "The Cartographer's Daughter", author: "June Okonkwo", year: 2020, status: "in-progress", progress: 0.16, format: "epub", pages: 368, addedDays: 27 },
  { id: "b08", title: "Riverlight Months", author: "Hadrien Cosse", year: 2017, status: "finished", format: "epub", pages: 224, addedDays: 142 },
  { id: "b09", title: "Notes on Disappearance", author: "Petra Wells", year: 2023, status: "unread", format: "epub", pages: 308, addedDays: 5 },
  { id: "b10", title: "The Long Tide", author: "Oluwaseun Kemi", year: 2022, status: "unread", format: "epub", pages: 416, addedDays: 9 },
  { id: "b11", title: "House at Six Hills", author: "Ines Roca", year: 2015, status: "finished", format: "epub", pages: 188, addedDays: 410 },
  { id: "b12", title: "Inland", author: "Mateusz Borowicz", year: 2021, status: "unread", format: "epub", pages: 272, addedDays: 14 },
  { id: "b13", title: "Atlas of Familiar Things", author: "Naia Linde", year: 2024, status: "unread", format: "epub", pages: 344, addedDays: 3 },
  { id: "b14", title: "What Remains After Sleep", author: "August Marlow", year: 2020, status: "finished", format: "epub", pages: 296, addedDays: 220 },
  { id: "b15", title: "Three Empty Rooms", author: "Lila Vasquez", year: 2019, status: "unread", format: "epub", pages: 232, addedDays: 36 },
  { id: "b16", title: "Counting Storms", author: "Bikram Shah", year: 2022, status: "unread", format: "epub", pages: 384, addedDays: 18 },
  { id: "b17", title: "Sister Clay", author: "Adaeze Nwankwo", year: 2024, status: "unread", format: "epub", pages: 412, addedDays: 6 },
  { id: "b18", title: "The Weight of Pages", author: "Henrik Sand", year: 2016, status: "finished", format: "epub", pages: 528, addedDays: 612 },
  { id: "b19", title: "Permanent Daylight", author: "Veda Kapoor", year: 2023, status: "unread", format: "epub", pages: 256, addedDays: 8 },
  { id: "b20", title: "Field Manual for Ghosts", author: "Cael Bishop", year: 2021, status: "unread", format: "pdf", pages: 176, addedDays: 32 },
  { id: "b21", title: "Provisional Coast", author: "Ines Roca", year: 2018, status: "unread", format: "epub", pages: 304, addedDays: 95 },
  { id: "b22", title: "Lantern Pulse: Below", author: "Yusra Al-Mansouri", year: 2023, status: "unread", format: "epub", pages: 422, addedDays: 12, series: { name: "Lantern Cycle", index: 2, total: 4 } },
  { id: "b23", title: "Lantern Pulse: Tower", author: "Yusra Al-Mansouri", year: 2024, status: "unread", format: "epub", pages: 446, addedDays: 7, series: { name: "Lantern Cycle", index: 3, total: 4 } },
  { id: "b24", title: "Slow Engines", author: "Mira Tanaka", year: 2020, status: "finished", format: "epub", pages: 282, addedDays: 178 },
  { id: "b25", title: "The Listening Room", author: "Cael Bishop", year: 2023, status: "unread", format: "epub", pages: 288, addedDays: 22 },
  { id: "b26", title: "Northern Static", author: "Saskia Brandt", year: 2019, status: "finished", format: "epub", pages: 376, addedDays: 320 },
  { id: "b27", title: "Honeycomb Cathedral", author: "Adaeze Nwankwo", year: 2021, status: "in-progress", progress: 0.61, format: "epub", pages: 504, addedDays: 41 },
  { id: "b28", title: "Without Anchor", author: "Tomás Fialho", year: 2024, status: "unread", format: "epub", pages: 248, addedDays: 4 },
];

export const SHELVES = {
  inProgress: BOOKS.filter((b) => b.status === "in-progress"),
  recentlyAdded: [...BOOKS].sort((a, b) => a.addedDays - b.addedDays).slice(0, 8),
  forgotten: BOOKS.filter((b) => b.status === "unread" && b.addedDays > 60).slice(0, 8),
  byYusra: BOOKS.filter((b) => b.author === "Yusra Al-Mansouri"),
  finishedThisYear: BOOKS.filter((b) => b.status === "finished").slice(0, 6),
};

export const USER_SHELVES = [
  { name: "Best of 2025", kind: "manual" as const, count: 12 },
  { name: "Slow reads", kind: "manual" as const, count: 7 },
  { name: "By Yusra Al-Mansouri", kind: "smart" as const, count: 3 },
  { name: "Lantern Cycle (incomplete)", kind: "smart" as const, count: 3 },
  { name: "Send to Kobo", kind: "device" as const, count: 5, device: "Kobo Libra 2", syncStatus: "pending" as const },
];

export const STATS = {
  totalBooks: 1247,
  read: 312,
  inProgress: 3,
  hoursThisYear: 184,
  pagesThisYear: 11420,
  finishedThisYear: 18,
};

export function bookHash(id: string): number {
  let h = 2166136261 >>> 0;
  for (let i = 0; i < id.length; i++) {
    h ^= id.charCodeAt(i);
    h = Math.imul(h, 16777619) >>> 0;
  }
  return h;
}

export function bookHue(id: string): number {
  return bookHash(id) % 360;
}

export function bookTier(id: string): 0 | 1 | 2 | 3 | 4 {
  return ((bookHash(id) >>> 8) % 5) as 0 | 1 | 2 | 3 | 4;
}

export function initials(author: string): string {
  return author
    .split(/\s+/)
    .map((p) => p.charAt(0))
    .filter(Boolean)
    .slice(0, 2)
    .join("")
    .toUpperCase();
}
