// FIFA World Cup 2026 TV Scoreboard Script
let currentSettings = null;
let timerInterval = null;
let secondsCounter = 0;
let previousScores = { team1: null, team2: null };
let goalBannerTimeout = null;

// DOM Elements
const overlayContainer = document.getElementById("overlay-container");
const compactFlag1 = document.getElementById("compact-flag1");
const compactCode1 = document.getElementById("compact-code1");
const compactScore1 = document.getElementById("compact-score1");
const compactFlag2 = document.getElementById("compact-flag2");
const compactCode2 = document.getElementById("compact-code2");
const compactScore2 = document.getElementById("compact-score2");
const compactTime = document.getElementById("compact-time");
const compactDot = document.getElementById("compact-dot");

const cardStage = document.getElementById("card-stage");
const cardFlag1 = document.getElementById("card-flag1");
const cardName1 = document.getElementById("card-name1");
const cardScore1 = document.getElementById("card-score1");
const cardFlag2 = document.getElementById("card-flag2");
const cardName2 = document.getElementById("card-name2");
const cardScore2 = document.getElementById("card-score2");

const goalBanner = document.getElementById("goal-banner");
const goalScorerFlag = document.getElementById("goal-scorer-flag");
const goalScorerName = document.getElementById("goal-scorer-name");

function applySettings(settings) {
    if (!settings) return;
    currentSettings = settings;

    // Apply Fonts and Base layout Mode
    if (settings.font_family) {
        document.documentElement.style.setProperty('--font-sans', settings.font_family + ', sans-serif');
    }

    // Toggle layouts
    if (overlayContainer) {
        if (settings.layout_mode === "full_time") {
            overlayContainer.className = "overlay-container mode-full-time";
        } else {
            overlayContainer.className = "overlay-container mode-in-progress";
        }
    }

    // Bind flags, names, codes, and scores
    if (compactFlag1) compactFlag1.src = settings.team1_flag || "https://flagcdn.com/w80/es.png";
    if (compactCode1) compactCode1.innerText = settings.team1_code || "ESP";
    if (compactScore1) compactScore1.innerText = settings.team1_score !== undefined ? settings.team1_score : 0;
    
    if (compactFlag2) compactFlag2.src = settings.team2_flag || "https://flagcdn.com/w80/us.png";
    if (compactCode2) compactCode2.innerText = settings.team2_code || "USA";
    if (compactScore2) compactScore2.innerText = settings.team2_score !== undefined ? settings.team2_score : 0;

    // Wide card binding
    if (cardStage) cardStage.innerText = settings.match_stage || "GROUP STAGE - GROUP A";
    if (cardFlag1) cardFlag1.src = settings.team1_flag || "https://flagcdn.com/w320/es.png";
    if (cardName1) cardName1.innerText = settings.team1_name || "Spain";
    if (cardScore1) cardScore1.innerText = settings.team1_score !== undefined ? settings.team1_score : 0;

    if (cardFlag2) cardFlag2.src = settings.team2_flag || "https://flagcdn.com/w320/us.png";
    if (cardName2) cardName2.innerText = settings.team2_name || "United States";
    if (cardScore2) cardScore2.innerText = settings.team2_score !== undefined ? settings.team2_score : 0;

    // Detect Goal Event (Score increments)
    const currentScore1 = settings.team1_score !== undefined ? parseInt(settings.team1_score) : 0;
    const currentScore2 = settings.team2_score !== undefined ? parseInt(settings.team2_score) : 0;

    if (previousScores.team1 !== null && currentScore1 > previousScores.team1) {
        triggerGoalCelebration(settings.team1_flag, settings.scorer_name || "GOAL!");
    }
    if (previousScores.team2 !== null && currentScore2 > previousScores.team2) {
        triggerGoalCelebration(settings.team2_flag, settings.scorer_name || "GOAL!");
    }

    // Sync baseline
    previousScores.team1 = currentScore1;
    previousScores.team2 = currentScore2;

    // Sync timer
    syncTimer(settings.match_time || "45:00", settings.timer_active);
}

function triggerGoalCelebration(flagUrl, scorerDetails) {
    if (goalBannerTimeout) {
        clearTimeout(goalBannerTimeout);
    }

    if (goalScorerFlag) goalScorerFlag.src = flagUrl || "";
    if (goalScorerName) goalScorerName.innerText = scorerDetails || "GOAL!";

    if (goalBanner) {
        goalBanner.classList.add("show");
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
        if (compactDot) compactDot.style.display = "inline-block";
        timerInterval = setInterval(() => {
            secondsCounter++;
            updateTimerDisplay();
        }, 1000);
    } else {
        if (compactDot) compactDot.style.display = "none";
    }
}

function updateTimerDisplay() {
    const mins = Math.floor(secondsCounter / 60);
    const secs = secondsCounter % 60;
    const formatted = `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
    if (compactTime) compactTime.innerText = formatted;
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
