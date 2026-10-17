// Which device to show: ?device=... in the URL, or the ESP32's default id
const DEVICE_ID = new URLSearchParams(location.search).get("device") ?? "homev1";
const REFRESH_MS = 30_000;

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
// Lagos is UTC+1 all year (no daylight saving), so chart ticks can line up with a fixed offset
const LAGOS_OFFSET = HOUR;

// Gap between chart ticks for each filter
const TICK_STEP = { "12h": 2 * HOUR, today: 3 * HOUR, this_week: DAY };

const BAR_HEIGHT = 36;
const LABEL_Y = 54;

const timeFormat = new Intl.DateTimeFormat("en-GB", {
    timeZone: "Africa/Lagos",
    hour: "2-digit",
    minute: "2-digit",
});
const clockFormat = new Intl.DateTimeFormat("en-GB", {
    timeZone: "Africa/Lagos",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
});
const weekdayFormat = new Intl.DateTimeFormat("en-GB", { timeZone: "Africa/Lagos", weekday: "short" });
const dayFormat = new Intl.DateTimeFormat("en-GB", {
    timeZone: "Africa/Lagos",
    weekday: "short",
    day: "numeric",
    month: "short",
});

const el = (id) => document.getElementById(id);

let summary = null; // last /summary response
let period = null; // last /period response
let filter = "12h";

// 5h 12m, or with seconds for the live clock: 5h 12m 09s
function formatDuration(ms, withSeconds = false) {
    const totalSeconds = Math.max(0, Math.floor(ms / 1000));
    const days = Math.floor(totalSeconds / 86400);
    const hours = Math.floor((totalSeconds % 86400) / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;

    const parts = [];
    if (days > 0) parts.push(`${days}d`);
    if (days > 0 || hours > 0) parts.push(`${hours}h`);
    parts.push(`${minutes}m`);
    if (withSeconds) parts.push(`${String(seconds).padStart(2, "0")}s`);
    return parts.join(" ");
}

// "14:02" for a time today, "Sat 10 Oct 14:02" for any other day
function formatWhen(time) {
    const sameDay = dayFormat.format(time) === dayFormat.format(Date.now());
    return sameDay ? timeFormat.format(time) : `${dayFormat.format(time)} ${timeFormat.format(time)}`;
}

async function fetchJson(path) {
    const res = await fetch(path);
    if (res.status === 404) throw new Error(`Unknown device "${DEVICE_ID}"`);
    if (!res.ok) throw new Error(`Server error (${res.status})`);
    return res.json();
}

function showError(error) {
    el("error").textContent = error ? `Couldn't update: ${error.message}` : "";
    el("error").hidden = !error;
    // If nothing has loaded yet, replace "Loading…" so the page doesn't look stuck
    if (error && !summary) el("state").textContent = "No data";
}

async function refresh() {
    try {
        [summary, period] = await Promise.all([
            fetchJson(`/summary/${DEVICE_ID}`),
            fetchJson(`/period/${DEVICE_ID}?filter=${filter}`),
        ]);
        showError(null);
        el("updated").textContent = `Updated ${clockFormat.format(Date.now())}`;
        renderStatus();
        renderTotals();
        renderChart();
    } catch (error) {
        showError(error);
    }
}

async function changeFilter(newFilter) {
    filter = newFilter;
    for (const button of document.querySelectorAll(".tabs button")) {
        button.classList.toggle("active", button.dataset.filter === filter);
    }

    try {
        const response = await fetchJson(`/period/${DEVICE_ID}?filter=${newFilter}`);
        // Ignore the response if another tab was picked while it was loading
        if (filter !== newFilter) return;
        period = response;
        showError(null);
        renderChart();
    } catch (error) {
        showError(error);
    }
}

// Runs every second, so the "for 2h 14m 09s" clock ticks without any requests
function renderStatus() {
    if (!summary) return;

    const on = summary.state === "on";
    const since = Date.parse(summary.since);

    document.body.dataset.state = summary.state;
    el("state").textContent = on ? "Power is ON" : "Power is OFF";
    el("duration").textContent = formatDuration(Date.now() - since, true);
    el("since").textContent = `${on ? "On" : "Off"} since ${formatWhen(since)}`;
}

function renderTotals() {
    el("today").textContent = formatDuration(summary.today_secs * 1000);
    el("week").textContent = formatDuration(summary.week_secs * 1000);
}

// The chart is one bar across the window: grey where the power was off, green where it was on.
// "Today" and "This week" stretch to the end of the day/week, so the rest is left empty,
// and so is any time before the device's first beat.
function renderChart() {
    if (!period) return;

    const svg = el("chart");
    const width = svg.clientWidth;
    const from = Date.parse(period.from);
    const now = Date.parse(period.to);
    const end = filter === "today" ? from + DAY : filter === "this_week" ? from + 7 * DAY : now;
    const x = (time) => ((time - from) / (end - from)) * width;

    const firstSeen = summary?.first_seen_at ? Date.parse(summary.first_seen_at) : from;
    const dataStart = Math.max(from, firstSeen);

    let shapes = `<rect class="no-data" x="0" y="0" width="${width}" height="${BAR_HEIGHT}" rx="4" />`;
    shapes += `<rect class="off" x="${x(dataStart)}" y="0" width="${x(now) - x(dataStart)}" height="${BAR_HEIGHT}" rx="4" />`;

    period.spans.forEach((span, i) => {
        const start = x(Date.parse(span.start));
        // At least 1px wide, so short spans are still visible
        const spanWidth = Math.max(1, x(Date.parse(span.end)) - start);
        shapes += `<rect class="on" data-index="${i}" x="${start}" y="0" width="${spanWidth}" height="${BAR_HEIGHT}" />`;
    });

    // Ticks fall on whole Lagos hours/days. Day labels sit in the middle of their day.
    const step = TICK_STEP[filter];
    const firstTick = Math.ceil((from + LAGOS_OFFSET) / step) * step - LAGOS_OFFSET;
    for (let tick = firstTick; tick < end; tick += step) {
        const isDay = step === DAY;
        const labelX = x(isDay ? tick + step / 2 : tick);
        // Skip labels that would be cut off at the edges
        if (labelX < 16 || labelX > width - 16) continue;

        const label = isDay ? weekdayFormat.format(tick) : timeFormat.format(tick);
        // A short tick under the bar, not a line through it, so it can't be mistaken for an outage
        shapes += `<line class="tick" x1="${x(tick)}" x2="${x(tick)}" y1="${BAR_HEIGHT + 2}" y2="${BAR_HEIGHT + 6}" />`;
        shapes += `<text x="${labelX}" y="${LABEL_Y}">${label}</text>`;
    }

    svg.innerHTML = shapes;
    showChartTotal();
}

// Default line under the chart: how much power there was in the selected window
function showChartTotal() {
    const totalMs = period.spans.reduce((sum, span) => sum + Date.parse(span.end) - Date.parse(span.start), 0);
    const windowName = { "12h": "in the last 12h", today: "today", this_week: "this week" }[filter];
    el("chart-info").textContent = `${formatDuration(totalMs)} of power ${windowName}. Tap a green bar for details.`;
}

// Tapping a green bar shows when that span started and ended
el("chart").addEventListener("click", (event) => {
    const index = event.target.dataset.index;
    if (index === undefined) {
        showChartTotal();
        return;
    }

    const span = period.spans[index];
    const start = Date.parse(span.start);
    const end = Date.parse(span.end);
    const endText = end === Date.parse(period.to) && summary?.state === "on" ? "now" : formatWhen(end);
    el("chart-info").textContent = `On ${formatWhen(start)} → ${endText} (${formatDuration(end - start)})`;
});

for (const button of document.querySelectorAll(".tabs button")) {
    button.addEventListener("click", () => changeFilter(button.dataset.filter));
}

// Refetch when the page comes back into view, e.g. switching back to the browser tab
document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") refresh();
});

window.addEventListener("resize", renderChart);

refresh();
setInterval(refresh, REFRESH_MS);
setInterval(renderStatus, 1000);
