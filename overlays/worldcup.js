// FIFA World Cup 2026 Scoreboard Overlay JS
let currentSettings = {};
let timerInterval = null;
let secondsCounter = 0;

const LOGO_URLS = {
    vertical_color: "https://static.wikia.nocookie.net/logopedia/images/5/51/FIFA_World_Cup_Canada_Mexico_USA_2026_Logo_With_World_Cup_%26_2026_Wordmarks_%26_Combined_Host_Countries_%28Dark_Gray_Text%29.png/revision/latest/scale-to-width-down/1000?cb=20260528181006",
    horizontal_color: "https://static.wikia.nocookie.net/logopedia/images/4/44/FWC26_CanMexUSA_hz.png/revision/latest/scale-to-width-down/1000?cb=20260604001303",
    vertical_black: "https://static.wikia.nocookie.net/logopedia/images/f/f7/FIFA_World_Cup_Canada_Mexico_USA_2026_Logo_With_World_Cup_%26_2026_Wordmarks_%28black_3%29.png/revision/latest/scale-to-width-down/1000?cb=20260521210834"
};

// DOM Elements
const widget = document.getElementById("scoreboard-widget");
const logoImg = document.getElementById("fwc-logo");
const team1Display = document.getElementById("team1-display");
const team1ScoreDisplay = document.getElementById("team1-score-display");
const team2Display = document.getElementById("team2-display");
const team2ScoreDisplay = document.getElementById("team2-score-display");
const timeDisplay = document.getElementById("time-display");
const liveDot = document.getElementById("live-dot");

function hexToRgba(hex, alpha) {
    if (!hex) return "rgba(15, 15, 22, 0.85)";
    hex = hex.replace('#', '');
    if (hex.length === 3) {
        hex = hex[0] + hex[0] + hex[1] + hex[1] + hex[2] + hex[2];
    }
    let r = parseInt(hex.substring(0, 2), 16) || 0;
    let g = parseInt(hex.substring(2, 4), 16) || 0;
    let b = parseInt(hex.substring(4, 6), 16) || 0;
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function applySettings(settings) {
    if (!settings) return;
    currentSettings = settings;

    // Apply Fonts and Dimensions
    document.documentElement.style.setProperty('--font-family', settings.font_family || 'Outfit, sans-serif');
    document.documentElement.style.setProperty('--font-size', (settings.font_size ? settings.font_size.replace('px', '') : '20') + 'px');
    
    // Background and Borders
    document.documentElement.style.setProperty('--bg-color-raw', hexToRgbValues(settings.bg_color || '#0f0f16'));
    document.documentElement.style.setProperty('--bg-opacity', settings.bg_opacity !== undefined ? settings.bg_opacity : 0.85);
    document.documentElement.style.setProperty('--border-color', settings.border_color || '#eab308');
    document.documentElement.style.setProperty('--border-width', (settings.border_width ? settings.border_width.replace('px', '') : '2') + 'px');
    document.documentElement.style.setProperty('--border-radius', (settings.border_radius ? settings.border_radius.replace('px', '') : '16') + 'px');
    document.documentElement.style.setProperty('--accent-color', settings.accent_color || '#eab308');
    document.documentElement.style.setProperty('--accent-glow', hexToRgba(settings.accent_color || '#eab308', 0.35));

    // Team Information
    if (team1Display) team1Display.innerText = settings.team1_name || "ESP";
    if (team1ScoreDisplay) team1ScoreDisplay.innerText = settings.team1_score !== undefined ? settings.team1_score : 0;
    if (team2Display) team2Display.innerText = settings.team2_name || "USA";
    if (team2ScoreDisplay) team2ScoreDisplay.innerText = settings.team2_score !== undefined ? settings.team2_score : 0;

    // Set layout based on logo variant
    const variant = settings.logo_variant || "horizontal_color";
    if (logoImg) {
        logoImg.src = LOGO_URLS[variant] || LOGO_URLS.horizontal_color;
    }

    if (variant === "horizontal_color") {
        widget.className = "scoreboard-container horizontal-layout";
    } else {
        widget.className = "scoreboard-container vertical-layout";
    }

    // Timer Sync
    syncTimer(settings.match_time || "00:00", settings.timer_active);
}

function hexToRgbValues(hex) {
    hex = hex.replace('#', '');
    if (hex.length === 3) {
        hex = hex[0] + hex[0] + hex[1] + hex[1] + hex[2] + hex[2];
    }
    let r = parseInt(hex.substring(0, 2), 16) || 0;
    let g = parseInt(hex.substring(2, 4), 16) || 0;
    let b = parseInt(hex.substring(4, 6), 16) || 0;
    return `${r}, ${g}, ${b}`;
}

function syncTimer(timeStr, isActive) {
    clearInterval(timerInterval);
    timerInterval = null;

    // Parse MM:SS
    const parts = timeStr.split(":");
    let mins = parseInt(parts[0]) || 0;
    let secs = parseInt(parts[1]) || 0;
    secondsCounter = mins * 60 + secs;

    updateTimerDisplay();

    if (isActive) {
        if (liveDot) liveDot.classList.add("active");
        timerInterval = setInterval(() => {
            secondsCounter++;
            updateTimerDisplay();
        }, 1000);
    } else {
        if (liveDot) liveDot.classList.remove("active");
    }
}

function updateTimerDisplay() {
    const mins = Math.floor(secondsCounter / 60);
    const secs = secondsCounter % 60;
    const formatted = `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
    if (timeDisplay) timeDisplay.innerText = formatted;
}

// Connect to SSE server stream
const sse = new EventSource('/api/stream');

sse.addEventListener('initial_config', (e) => {
    try {
        const data = JSON.parse(e.data);
        if (data.tools && data.tools.worldcup && data.tools.worldcup.settings) {
            applySettings(data.tools.worldcup.settings);
        }
    } catch (err) {
        console.error("Error loading initial worldcup settings:", err);
    }
});

sse.addEventListener('config_update', (e) => {
    try {
        const data = JSON.parse(e.data);
        if (data.tools && data.tools.worldcup && data.tools.worldcup.settings) {
            applySettings(data.tools.worldcup.settings);
        }
    } catch (err) {
        console.error("Error updating worldcup settings:", err);
    }
});
