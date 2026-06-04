// FIFA World Cup 2026 TV Scoreboard Script
let currentSettings = null;
let timerInterval = null;
let secondsCounter = 0;
let previousScores = { team1: null, team2: null };
let goalBannerTimeout = null;

// DOM Elements
const team1Code = document.getElementById("team1-code-display");
const team1Score = document.getElementById("team1-score-box");
const team2Code = document.getElementById("team2-code-display");
const team2Score = document.getElementById("team2-score-box");
const timeDisplay = document.getElementById("time-display");
const timerDot = document.getElementById("timer-dot");
const goalBanner = document.getElementById("goal-banner");
const goalTeamLabel = document.getElementById("goal-team-label");

function applySettings(settings) {
    if (!settings) return;
    currentSettings = settings;

    // Apply basic font settings dynamically
    if (settings.font_family) {
        document.documentElement.style.setProperty('--font-family-title', settings.font_family + ', sans-serif');
    }

    // Set Team Codes and scores
    if (team1Code) team1Code.innerText = settings.team1_name || "ESP";
    if (team1Score) team1Score.innerText = settings.team1_score !== undefined ? settings.team1_score : 0;
    
    if (team2Code) team2Code.innerText = settings.team2_name || "USA";
    if (team2Score) team2Score.innerText = settings.team2_score !== undefined ? settings.team2_score : 0;

    // Detect Goal Event (Score increments)
    const currentScore1 = settings.team1_score !== undefined ? parseInt(settings.team1_score) : 0;
    const currentScore2 = settings.team2_score !== undefined ? parseInt(settings.team2_score) : 0;

    if (previousScores.team1 !== null && currentScore1 > previousScores.team1) {
        triggerGoalCelebration(settings.team1_name || "ESP", "left");
    }
    if (previousScores.team2 !== null && currentScore2 > previousScores.team2) {
        triggerGoalCelebration(settings.team2_name || "USA", "right");
    }

    // Update previous scores baseline
    previousScores.team1 = currentScore1;
    previousScores.team2 = currentScore2;

    // Timer setup
    syncTimer(settings.match_time || "00:00", settings.timer_active);
}

function triggerGoalCelebration(teamName, side) {
    if (goalBannerTimeout) {
        clearTimeout(goalBannerTimeout);
    }

    if (goalTeamLabel) {
        goalTeamLabel.innerText = teamName;
    }

    if (goalBanner) {
        // Set dynamic colors based on side or tournament colors
        if (side === "left") {
            goalBanner.style.background = "linear-gradient(135deg, #ff3d00 0%, #d500f9 100%)"; // coral to purple
        } else {
            goalBanner.style.background = "linear-gradient(135deg, #00b0ff 0%, #00e676 100%)"; // cyan to lime green
        }
        
        goalBanner.classList.add("show");

        // Retract banner after 6 seconds
        goalBannerTimeout = setTimeout(() => {
            goalBanner.classList.remove("show");
        }, 6000);
    }
}

function syncTimer(timeStr, isActive) {
    clearInterval(timerInterval);
    timerInterval = null;

    const parts = timeStr.split(":");
    let mins = parseInt(parts[0]) || 0;
    let secs = parseInt(parts[1]) || 0;
    secondsCounter = mins * 60 + secs;

    updateTimerDisplay();

    if (isActive) {
        if (timerDot) timerDot.classList.add("active");
        timerInterval = setInterval(() => {
            secondsCounter++;
            updateTimerDisplay();
        }, 1000);
    } else {
        if (timerDot) timerDot.classList.remove("active");
    }
}

function updateTimerDisplay() {
    const mins = Math.floor(secondsCounter / 60);
    const secs = secondsCounter % 60;
    const formatted = `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
    if (timeDisplay) timeDisplay.innerText = formatted;
}

// Connect to SSE stream
const sse = new EventSource('/api/stream');

sse.addEventListener('initial_config', (e) => {
    try {
        const data = JSON.parse(e.data);
        if (data.tools && data.tools.worldcup && data.tools.worldcup.settings) {
            applySettings(data.tools.worldcup.settings);
        }
    } catch (err) {
        console.error("Error loading initial config:", err);
    }
});

sse.addEventListener('config_update', (e) => {
    try {
        const data = JSON.parse(e.data);
        if (data.tools && data.tools.worldcup && data.tools.worldcup.settings) {
            applySettings(data.tools.worldcup.settings);
        }
    } catch (err) {
        console.error("Error updating config:", err);
    }
});
